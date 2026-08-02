/**
 * Payment Methods Resource
 *
 * Manage payment methods for customers.
 */

import type {
  PaymentMethod,
  PaymentMethodParams,
  ApiResponse,
} from '../types';

export class PaymentMethods {
  private client: import('../client').PaymentOrchestra;

  constructor(client: import('../client').PaymentOrchestra) {
    this.client = client;
  }

  /**
   * Create a new payment method
   *
   * @example
   * ```typescript
   * const paymentMethod = await client.paymentMethods.create({
   *   type: 'card',
   *   card: {
   *     number: '4242424242424242',
   *     exp_month: 12,
   *     exp_year: 2025,
   *     cvc: '123',
   *   },
   *   billing_details: {
   *     name: 'John Doe',
   *     email: 'john@example.com',
   *   },
   * });
   * ```
   */
  async create(params: PaymentMethodParams): Promise<PaymentMethod> {
    const response = await this.client.request<PaymentMethod>(
      'POST',
      '/payment_methods',
      params as unknown as Record<string, unknown>
    );

    return response.data;
  }

  /**
   * Retrieve a payment method by ID
   *
   * @example
   * ```typescript
   * const paymentMethod = await client.paymentMethods.retrieve('pm_1234567890');
   * ```
   */
  async retrieve(id: string): Promise<PaymentMethod> {
    const response = await this.client.request<PaymentMethod>(
      'GET',
      `/payment_methods/${id}`
    );

    return response.data;
  }

  /**
   * Update a payment method
   *
   * @example
   * ```typescript
   * const paymentMethod = await client.paymentMethods.update('pm_1234567890', {
   *   billing_details: {
   *     name: 'Jane Doe',
   *   },
   * });
   * ```
   */
  async update(
    id: string,
    params: { billing_details?: Record<string, unknown> }
  ): Promise<PaymentMethod> {
    const response = await this.client.request<PaymentMethod>(
      'POST',
      `/payment_methods/${id}`,
      params as unknown as Record<string, unknown>
    );

    return response.data;
  }

  /**
   * Detach a payment method from a customer
   *
   * @example
   * ```typescript
   * const paymentMethod = await client.paymentMethods.detach('pm_1234567890');
   * ```
   */
  async detach(id: string): Promise<PaymentMethod> {
    const response = await this.client.request<PaymentMethod>(
      'POST',
      `/payment_methods/${id}/detach`
    );

    return response.data;
  }

  /**
   * List payment methods for a customer
   *
   * @example
   * ```typescript
   * const paymentMethods = await client.paymentMethods.list({
   *   customer: 'cus_1234567890',
   *   type: 'card',
   * });
   * ```
   */
  async list(params: {
    customer: string;
    type?: string;
    limit?: number;
    starting_after?: string;
    ending_before?: string;
  }): Promise<{ data: PaymentMethod[]; has_more: boolean }> {
    const queryParams = new URLSearchParams();
    queryParams.append('customer', params.customer);

    if (params.type) queryParams.append('type', params.type);
    if (params.limit) queryParams.append('limit', String(params.limit));
    if (params.starting_after)
      queryParams.append('starting_after', params.starting_after);
    if (params.ending_before)
      queryParams.append('ending_before', params.ending_before);

    const query = queryParams.toString();
    const path = `/payment_methods?${query}`;

    const response = await this.client.request<{
      data: PaymentMethod[];
      has_more: boolean;
    }>('GET', path);

    return response.data;
  }

  /**
   * Create a card token for secure card handling
   * Card data is tokenized and never touches your server
   *
   * @example
   * ```typescript
   * const token = await client.paymentMethods.tokenizeCard({
   *   number: '4242424242424242',
   *   exp_month: 12,
   *   exp_year: 2025,
   *   cvc: '123',
   * });
   *
   * // Use the token to create a payment method
   * const paymentMethod = await client.paymentMethods.create({
   *   type: 'card',
   *   card: {
   *     token: token.id,
   *   },
   * });
   * ```
   */
  async tokenizeCard(card: {
    number: string;
    exp_month: number;
    exp_year: number;
    cvc: string;
  }): Promise<{ id: string; card: { last4: string; brand: string } }> {
    return this.client.tokenizeCard({
      ...card,
      exp_month: card.exp_month,
      exp_year: card.exp_year,
      cvc: card.cvc,
    });
  }
}
