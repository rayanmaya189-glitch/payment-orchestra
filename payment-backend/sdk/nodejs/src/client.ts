/**
 * Payment Orchestration Platform Client
 */

import axios, { AxiosInstance, AxiosError } from 'axios';
import { PaymentIntents } from './resources/payment-intents';
import { GatewayProfiles } from './resources/gateway-profiles';
import { RoutingPolicies } from './resources/routing-policies';
import { Webhooks } from './resources/webhooks';
import { PaymentOrchestraError, AuthenticationError, RateLimitError } from './errors';

export interface PaymentOrchestraConfig {
  apiKey: string;
  secretKey: string;
  environment?: 'sandbox' | 'production';
  baseUrl?: string;
  timeout?: number;
  maxRetries?: number;
}

export class PaymentOrchestra {
  private readonly httpClient: AxiosInstance;
  private readonly _paymentIntents: PaymentIntents;
  private readonly _gatewayProfiles: GatewayProfiles;
  private readonly _routingPolicies: RoutingPolicies;
  private readonly _webhooks: Webhooks;

  constructor(config: PaymentOrchestraConfig) {
    const {
      apiKey,
      secretKey,
      environment = 'sandbox',
      baseUrl,
      timeout = 30000,
      maxRetries = 3,
    } = config;

    const defaultBaseUrl = environment === 'production'
      ? 'https://api.paymentorchestra.com'
      : 'https://sandbox.api.paymentorchestra.com';

    this.httpClient = axios.create({
      baseURL: baseUrl || defaultBaseUrl,
      timeout,
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${apiKey}`,
        'X-Secret-Key': secretKey,
        'X-SDK-Version': '1.0.0',
        'X-SDK-Name': 'nodejs',
      },
    });

    // Add retry interceptor
    this.httpClient.interceptors.response.use(
      (response) => response,
      async (error: AxiosError) => {
        if (error.response?.status === 429) {
          const retryAfter = error.response.headers['retry-after'];
          if (retryAfter) {
            await new Promise(resolve => setTimeout(resolve, parseInt(retryAfter) * 1000));
            return this.httpClient.request(error.config!);
          }
        }
        throw error;
      }
    );

    // Add error interceptor
    this.httpClient.interceptors.response.use(
      (response) => response,
      (error: AxiosError) => {
        if (error.response) {
          const { status, data } = error.response;
          const message = (data as any)?.error?.message || error.message;

          switch (status) {
            case 401:
              throw new AuthenticationError(message);
            case 429:
              throw new RateLimitError(message);
            default:
              throw new PaymentOrchestraError(status, message, data);
          }
        }
        throw error;
      }
    );

    this._paymentIntents = new PaymentIntents(this.httpClient);
    this._gatewayProfiles = new GatewayProfiles(this.httpClient);
    this._routingPolicies = new RoutingPolicies(this.httpClient);
    this._webhooks = new Webhooks(this.httpClient);
  }

  get paymentIntents(): PaymentIntents {
    return this._paymentIntents;
  }

  get gatewayProfiles(): GatewayProfiles {
    return this._gatewayProfiles;
  }

  get routingPolicies(): RoutingPolicies {
    return this._routingPolicies;
  }

  get webhooks(): Webhooks {
    return this._webhooks;
  }
}
