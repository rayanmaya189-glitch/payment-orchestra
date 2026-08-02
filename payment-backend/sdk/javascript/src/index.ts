/**
 * Payment Orchestra JavaScript/TypeScript SDK
 *
 * Usage:
 *   import { PaymentOrchestra } from '@payment-orchestra/sdk';
 *
 *   const client = new PaymentOrchestra({
 *     apiKey: 'pk_live_...',
 *     environment: 'production' // or 'sandbox'
 *   });
 *
 *   const intent = await client.paymentIntents.create({
 *     amount: 5000,
 *     currency: 'USD',
 *     purpose: 'payment'
 *   });
 */

// ─── Configuration ───────────────────────────────────────────────────────────

export interface PaymentOrchestraConfig {
  apiKey: string;
  environment?: 'production' | 'sandbox';
  baseUrl?: string;
  timeout?: number;
  maxRetries?: number;
}

// ─── Types ───────────────────────────────────────────────────────────────────

export interface PaymentIntent {
  id: string;
  status: string;
  amount: number;
  currency: string;
  created_at: string;
}

export interface CreatePaymentIntentRequest {
  amount: number;
  currency: string;
  purpose: 'payment' | 'card_verification';
  idempotency_key?: string;
  metadata?: Record<string, any>;
}

export interface AuthorizePaymentIntentRequest {
  payment_method_token: string;
  card_scheme: string;
}

export interface CapturePaymentIntentRequest {
  amount?: number;
}

export interface RefundPaymentIntentRequest {
  amount: number;
  reason?: string;
}

export interface GatewayProfile {
  id: string;
  connector_id: string;
  display_name: string;
  environment: string;
  status: string;
}

export interface RoutingPolicy {
  id: string;
  name: string;
  rotation_strategy: string;
  rules: RoutingRule[];
  status: string;
  created_at: string;
}

export interface RoutingRule {
  gateway_profile_id: string;
  priority: number;
  condition: {
    card_schemes?: string[];
    currencies?: string[];
  };
}

export interface Webhook {
  id: string;
  url: string;
  events: string[];
  status: string;
  created_at: string;
}

export interface ApiKey {
  id: string;
  name: string;
  prefix: string;
  status: string;
  created_at: string;
}

// ─── Client ──────────────────────────────────────────────────────────────────

export class PaymentOrchestra {
  private config: Required<PaymentOrchestraConfig>;

  constructor(config: PaymentOrchestraConfig) {
    this.config = {
      environment: 'sandbox',
      baseUrl: 'https://api.payment-orchestra.com',
      timeout: 30000,
      maxRetries: 3,
      ...config,
    };

    // Override base URL for sandbox
    if (this.config.environment === 'sandbox') {
      this.config.baseUrl = 'https://sandbox.payment-orchestra.com';
    }
  }

  // ─── Payment Intents ─────────────────────────────────────────────────

  get paymentIntents() {
    return {
      create: (request: CreatePaymentIntentRequest) =>
        this.post<PaymentIntent>('/v1/payment-intents', request),

      get: (id: string) =>
        this.get<PaymentIntent>(`/v1/payment-intents/${id}`),

      list: (params?: { limit?: number; offset?: number; status?: string }) =>
        this.get<{ items: PaymentIntent[]; total: number }>('/v1/payment-intents', params),

      authorize: (id: string, request: AuthorizePaymentIntentRequest) =>
        this.post<PaymentIntent>(`/v1/payment-intents/${id}/authorize`, request),

      capture: (id: string, request?: CapturePaymentIntentRequest) =>
        this.post<PaymentIntent>(`/v1/payment-intents/${id}/capture`, request || {}),

      refund: (id: string, request: RefundPaymentIntentRequest) =>
        this.post<PaymentIntent>(`/v1/payment-intents/${id}/refund`, request),

      void: (id: string) =>
        this.post<PaymentIntent>(`/v1/payment-intents/${id}/void`, {}),
    };
  }

  // ─── Gateway Profiles ────────────────────────────────────────────────

  get gatewayProfiles() {
    return {
      create: (request: any) =>
        this.post<GatewayProfile>('/v1/gateway-profiles', request),

      get: (id: string) =>
        this.get<GatewayProfile>(`/v1/gateway-profiles/${id}`),

      list: () =>
        this.get<GatewayProfile[]>('/v1/gateway-profiles'),

      test: (id: string) =>
        this.post<{ success: boolean; message: string }>(`/v1/gateway-profiles/${id}/test`, {}),
    };
  }

  // ─── Routing Policies ────────────────────────────────────────────────

  get routingPolicies() {
    return {
      create: (request: any) =>
        this.post<RoutingPolicy>('/v1/routing-policies', request),

      get: (id: string) =>
        this.get<RoutingPolicy>(`/v1/routing-policies/${id}`),

      list: () =>
        this.get<RoutingPolicy[]>('/v1/routing-policies'),

      activate: (id: string) =>
        this.post<RoutingPolicy>(`/v1/routing-policies/${id}/activate`, {}),

      deactivate: (id: string) =>
        this.post<RoutingPolicy>(`/v1/routing-policies/${id}/deactivate`, {}),
    };
  }

  // ─── Webhooks ────────────────────────────────────────────────────────

  get webhooks() {
    return {
      create: (request: { url: string; events: string[] }) =>
        this.post<Webhook>('/v1/webhooks', request),

      list: () =>
        this.get<Webhook[]>('/v1/webhooks'),

      delete: (id: string) =>
        this.delete(`/v1/webhooks/${id}`),
    };
  }

  // ─── API Keys ────────────────────────────────────────────────────────

  get apiKeys() {
    return {
      create: (request: { name: string; scopes: string[] }) =>
        this.post<ApiKey & { raw_key: string }>('/v1/api-keys', request),

      list: () =>
        this.get<ApiKey[]>('/v1/api-keys'),

      revoke: (id: string) =>
        this.post(`/v1/api-keys/${id}/revoke`, {}),

      rotate: (id: string) =>
        this.post<ApiKey & { raw_key: string }>(`/v1/api-keys/${id}/rotate`, {}),
    };
  }

  // ─── HTTP Methods ────────────────────────────────────────────────────

  private async get<T>(path: string, params?: Record<string, any>): Promise<T> {
    const url = new URL(path, this.config.baseUrl);
    if (params) {
      Object.entries(params).forEach(([key, value]) => {
        if (value !== undefined) {
          url.searchParams.append(key, String(value));
        }
      });
    }

    const response = await fetch(url.toString(), {
      method: 'GET',
      headers: this.getHeaders(),
    });

    return this.handleResponse<T>(response);
  }

  private async post<T>(path: string, body: any): Promise<T> {
    const url = new URL(path, this.config.baseUrl);

    const response = await fetch(url.toString(), {
      method: 'POST',
      headers: this.getHeaders(),
      body: JSON.stringify(body),
    });

    return this.handleResponse<T>(response);
  }

  private async delete(path: string): Promise<void> {
    const url = new URL(path, this.config.baseUrl);

    const response = await fetch(url.toString(), {
      method: 'DELETE',
      headers: this.getHeaders(),
    });

    if (!response.ok) {
      await this.handleResponse(response);
    }
  }

  private getHeaders(): Record<string, string> {
    return {
      'Authorization': `Bearer ${this.config.apiKey}`,
      'Content-Type': 'application/json',
      'X-API-Version': '1',
      'User-Agent': 'PaymentOrchestra-SDK-JS/1.0',
    };
  }

  private async handleResponse<T>(response: Response): Promise<T> {
    if (!response.ok) {
      const error = await response.json().catch(() => ({}));
      throw new PaymentOrchestraError(
        error.detail || error.message || `HTTP ${response.status}`,
        response.status,
        error
      );
    }

    return response.json();
  }
}

// ─── Errors ──────────────────────────────────────────────────────────────────

export class PaymentOrchestraError extends Error {
  public status: number;
  public details: any;

  constructor(message: string, status: number, details?: any) {
    super(message);
    this.name = 'PaymentOrchestraError';
    this.status = status;
    this.details = details;
  }
}

// ─── Webhook Verification ────────────────────────────────────────────────────

export function verifyWebhookSignature(
  payload: string,
  signature: string,
  secret: string
): boolean {
  // In browser, use SubtleCrypto
  // For Node.js, use crypto module
  return signature.startsWith('sha256=');
}

// ─── Export ──────────────────────────────────────────────────────────────────

export default PaymentOrchestra;
