/**
 * Payment Orchestra React Native Client
 *
 * Main client class for interacting with the Payment Orchestra API.
 * Provides access to all resources and native payment methods.
 */

import { Platform } from 'react-native';
import { PaymentIntents } from './resources/payment-intents';
import { PaymentMethods } from './resources/payment-methods';
import { Customers } from './resources/customers';
import { Refunds } from './resources/refunds';
import { ApplePay } from './apple-pay';
import { GooglePay } from './google-pay';
import { PaymentOrchestraError, ErrorCode } from './errors';
import type { Environment, PaymentOrchestraConfig, ApiResponse } from './types';

// Native module imports (will be linked via react-native.config.js)
const PaymentOrchestraNative = require('./native').default;

export interface ClientConfig extends PaymentOrchestraConfig {
  /** API key for authentication */
  apiKey: string;
  /** Environment: 'sandbox' or 'production' */
  environment?: Environment;
  /** Base URL for API (optional, defaults based on environment) */
  baseUrl?: string;
  /** Request timeout in milliseconds */
  timeout?: number;
  /** Enable verbose logging */
  debug?: boolean;
}

export class PaymentOrchestra {
  /** Payment Intents resource */
  public readonly paymentIntents: PaymentIntents;

  /** Payment Methods resource */
  public readonly paymentMethods: PaymentMethods;

  /** Customers resource */
  public readonly customers: Customers;

  /** Refunds resource */
  public readonly refunds: Refunds;

  /** Apple Pay integration (iOS only) */
  public readonly applePay: ApplePay;

  /** Google Pay integration (Android only) */
  public readonly googlePay: GooglePay;

  private readonly apiKey: string;
  private readonly baseUrl: string;
  private readonly timeout: number;
  private readonly debug: boolean;

  constructor(config: ClientConfig) {
    if (!config.apiKey) {
      throw new PaymentOrchestraError(
        'API key is required',
        ErrorCode.INVALID_REQUEST
      );
    }

    this.apiKey = config.apiKey;
    this.debug = config.debug ?? false;
    this.timeout = config.timeout ?? 30000;

    // Set base URL based on environment
    const environment = config.environment ?? 'sandbox';
    if (config.baseUrl) {
      this.baseUrl = config.baseUrl;
    } else {
      this.baseUrl =
        environment === 'production'
          ? 'https://api.paymentorchestra.com'
          : 'https://sandbox.paymentorchestra.com';
    }

    // Initialize resources
    this.paymentIntents = new PaymentIntents(this);
    this.paymentMethods = new PaymentMethods(this);
    this.customers = new Customers(this);
    this.refunds = new Refunds(this);

    // Initialize native payment methods
    this.applePay = new ApplePay(this);
    this.googlePay = new GooglePay(this);

    this.log('PaymentOrchestra initialized', { environment, baseUrl: this.baseUrl });
  }

  /**
   * Make an authenticated API request
   */
  async request<T>(
    method: string,
    path: string,
    body?: Record<string, unknown>
  ): Promise<ApiResponse<T>> {
    const url = `${this.baseUrl}/v1${path}`;

    this.log('API Request', { method, path, body });

    try {
      const response = await PaymentOrchestraNative.makeRequest({
        method,
        url,
        headers: {
          'Authorization': `Bearer ${this.apiKey}`,
          'Content-Type': 'application/json',
          'X-Platform': Platform.OS,
          'X-SDK-Version': '1.0.0',
        },
        body: body ? JSON.stringify(body) : undefined,
        timeout: this.timeout,
      });

      const data = JSON.parse(response);
      this.log('API Response', { status: data.status, path });

      if (data.error) {
        throw new PaymentOrchestraError(
          data.error.message || 'API request failed',
          data.error.code || ErrorCode.API_ERROR,
          data.error
        );
      }

      return data;
    } catch (error) {
      if (error instanceof PaymentOrchestraError) {
        throw error;
      }

      this.log('API Error', { error: String(error) });

      throw new PaymentOrchestraError(
        `Network request failed: ${String(error)}`,
        ErrorCode.NETWORK_ERROR
      );
    }
  }

  /**
   * Tokenize a card for secure storage
   * Card data is tokenized on-device and never sent to your server
   */
  async tokenizeCard(card: {
    number: string;
    exp_month: number;
    exp_year: number;
    cvc: string;
    name?: string;
  }): Promise<{ id: string; last4: string; brand: string }> {
    this.log('Tokenizing card', { last4: card.number.slice(-4) });

    // Use native module for secure tokenization
    const token = await PaymentOrchestraNative.tokenizeCard({
      number: card.number,
      expMonth: card.exp_month,
      expYear: card.exp_year,
      cvc: card.cvc,
      name: card.name,
    });

    return token;
  }

  /**
   * Verify a card using 3D Secure
   */
  async verify3DSecure(
    paymentIntentId: string,
    cardToken: string
  ): Promise<{ verified: boolean; redirectUrl?: string }> {
    this.log('Verifying 3DS', { paymentIntentId });

    const result = await this.paymentIntents.confirm(paymentIntentId, {
      payment_method: {
        token: cardToken,
        three_d_secure: true,
      },
    });

    return {
      verified: result.status === 'succeeded',
      redirectUrl: result.next_action?.redirect_to_url,
    };
  }

  /**
   * Get device information for fraud detection
   */
  async getDeviceInfo(): Promise<Record<string, unknown>> {
    return PaymentOrchestraNative.getDeviceInfo();
  }

  /**
   * Validate environment setup
   */
  async validateSetup(): Promise<{ valid: boolean; issues: string[] }> {
    return PaymentOrchestraNative.validateSetup();
  }

  /**
   * Enable debug logging
   */
  setDebug(enabled: boolean): void {
    (this as { debug: boolean }).debug = enabled;
  }

  /**
   * Internal logging utility
   */
  private log(message: string, data?: Record<string, unknown>): void {
    if (this.debug) {
      console.log(`[PaymentOrchestra] ${message}`, data ?? '');
    }
  }
}

export default PaymentOrchestra;
