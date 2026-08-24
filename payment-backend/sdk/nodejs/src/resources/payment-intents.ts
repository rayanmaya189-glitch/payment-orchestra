/**
 * Payment Intents resource for Payment Orchestration Platform SDK
 */

import { AxiosInstance } from 'axios';
import {
  CreatePaymentIntentParams,
  UpdatePaymentIntentParams,
  PaymentIntent,
  AuthorizePaymentIntentParams,
  CapturePaymentIntentParams,
  RefundPaymentIntentParams,
  PaymentIntentListParams,
} from '../types/payment-intent';
import { PaginatedResponse } from '../types/common';

export class PaymentIntents {
  private readonly httpClient: AxiosInstance;

  constructor(httpClient: AxiosInstance) {
    this.httpClient = httpClient;
  }

  /**
   * Create a new payment intent
   */
  async create(params: CreatePaymentIntentParams): Promise<PaymentIntent> {
    const response = await this.httpClient.post<PaymentIntent>(
      '/v1/payment_intents',
      params
    );
    return response.data;
  }

  /**
   * Retrieve a payment intent by ID
   */
  async retrieve(id: string): Promise<PaymentIntent> {
    const response = await this.httpClient.get<PaymentIntent>(
      `/v1/payment_intents/${id}`
    );
    return response.data;
  }

  /**
   * Update a payment intent
   */
  async update(
    id: string,
    params: UpdatePaymentIntentParams
  ): Promise<PaymentIntent> {
    const response = await this.httpClient.patch<PaymentIntent>(
      `/v1/payment_intents/${id}`,
      params
    );
    return response.data;
  }

  /**
   * Authorize a payment intent
   */
  async authorize(
    id: string,
    params?: AuthorizePaymentIntentParams
  ): Promise<PaymentIntent> {
    const response = await this.httpClient.post<PaymentIntent>(
      `/v1/payment_intents/${id}/authorize`,
      params || {}
    );
    return response.data;
  }

  /**
   * Capture a payment intent
   */
  async capture(
    id: string,
    params?: CapturePaymentIntentParams
  ): Promise<PaymentIntent> {
    const response = await this.httpClient.post<PaymentIntent>(
      `/v1/payment_intents/${id}/capture`,
      params || {}
    );
    return response.data;
  }

  /**
   * Cancel a payment intent
   */
  async cancel(id: string): Promise<PaymentIntent> {
    const response = await this.httpClient.post<PaymentIntent>(
      `/v1/payment_intents/${id}/cancel`
    );
    return response.data;
  }

  /**
   * Refund a payment intent
   */
  async refund(
    id: string,
    params?: RefundPaymentIntentParams
  ): Promise<PaymentIntent> {
    const response = await this.httpClient.post<PaymentIntent>(
      `/v1/payment_intents/${id}/refund`,
      params || {}
    );
    return response.data;
  }

  /**
   * List payment intents
   */
  async list(
    params?: PaymentIntentListParams
  ): Promise<PaginatedResponse<PaymentIntent>> {
    const response = await this.httpClient.get<PaginatedResponse<PaymentIntent>>(
      '/v1/payment_intents',
      { params }
    );
    return response.data;
  }

  /**
   * Confirm a payment intent (for manual confirmation)
   */
  async confirm(id: string): Promise<PaymentIntent> {
    const response = await this.httpClient.post<PaymentIntent>(
      `/v1/payment_intents/${id}/confirm`
    );
    return response.data;
  }

  /**
   * Get payment intent by order ID
   */
  async retrieveByOrderId(orderId: string): Promise<PaymentIntent> {
    const response = await this.httpClient.get<PaymentIntent>(
      `/v1/payment_intents/order/${orderId}`
    );
    return response.data;
  }
}
