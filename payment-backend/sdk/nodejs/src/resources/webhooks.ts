/**
 * Webhooks resource for Payment Orchestration Platform SDK
 */

import { AxiosInstance } from 'axios';
import { createHmac } from 'crypto';
import {
  CreateWebhookParams,
  UpdateWebhookParams,
  Webhook,
  WebhookListParams,
  WebhookEvent,
  WebhookDelivery,
} from '../types/webhook';
import { PaginatedResponse } from '../types/common';

export class Webhooks {
  private readonly httpClient: AxiosInstance;

  constructor(httpClient: AxiosInstance) {
    this.httpClient = httpClient;
  }

  /**
   * Create a new webhook
   */
  async create(params: CreateWebhookParams): Promise<Webhook> {
    const response = await this.httpClient.post<Webhook>(
      '/v1/webhooks',
      params
    );
    return response.data;
  }

  /**
   * Retrieve a webhook by ID
   */
  async retrieve(id: string): Promise<Webhook> {
    const response = await this.httpClient.get<Webhook>(
      `/v1/webhooks/${id}`
    );
    return response.data;
  }

  /**
   * Update a webhook
   */
  async update(
    id: string,
    params: UpdateWebhookParams
  ): Promise<Webhook> {
    const response = await this.httpClient.patch<Webhook>(
      `/v1/webhooks/${id}`,
      params
    );
    return response.data;
  }

  /**
   * Delete a webhook
   */
  async delete(id: string): Promise<void> {
    await this.httpClient.delete(`/v1/webhooks/${id}`);
  }

  /**
   * List webhooks
   */
  async list(
    params?: WebhookListParams
  ): Promise<PaginatedResponse<Webhook>> {
    const response = await this.httpClient.get<PaginatedResponse<Webhook>>(
      '/v1/webhooks',
      { params }
    );
    return response.data;
  }

  /**
   * Get webhook deliveries
   */
  async getDeliveries(
    webhookId: string,
    limit?: number
  ): Promise<WebhookDelivery[]> {
    const response = await this.httpClient.get<WebhookDelivery[]>(
      `/v1/webhooks/${webhookId}/deliveries`,
      { params: { limit } }
    );
    return response.data;
  }

  /**
   * Retry a failed delivery
   */
  async retryDelivery(
    webhookId: string,
    deliveryId: string
  ): Promise<WebhookDelivery> {
    const response = await this.httpClient.post<WebhookDelivery>(
      `/v1/webhooks/${webhookId}/deliveries/${deliveryId}/retry`
    );
    return response.data;
  }

  /**
   * Verify webhook signature
   */
  static verifySignature(
    payload: string,
    signature: string,
    secret: string
  ): boolean {
    const expectedSignature = createHmac('sha256', secret)
      .update(payload)
      .digest('hex');
    return signature === expectedSignature;
  }

  /**
   * Parse webhook event from raw payload
   */
  static parseEvent(payload: string): WebhookEvent {
    return JSON.parse(payload) as WebhookEvent;
  }
}
