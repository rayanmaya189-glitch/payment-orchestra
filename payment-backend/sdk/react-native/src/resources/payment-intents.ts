/**
 * Payment Intents Resource
 *
 * Manage payment intents for processing payments.
 */

import type {
  PaymentIntent,
  CreatePaymentIntentParams,
  ConfirmPaymentIntentParams,
  ApiResponse,
} from '../types';

export class PaymentIntents {
  private client: import('../client').PaymentOrchestra;

  constructor(client: import('../client').PaymentOrchestra) {
    this.client = client;
  }

  /**
   * Create a new payment intent
   *
   * @example
   * ```typescript
   * const intent = await client.paymentIntents.create({
   *   amount: 5000, // $50.00
   *   currency: 'usd',
   *   payment_method_types: ['card'],
   * });
   * ```
   */
  async create(params: CreatePaymentIntentParams): Promise<PaymentIntent> {
    const response = await this.client.request<PaymentIntent>(
      'POST',
      '/payment_intents',
      params as unknown as Record<string, unknown>
    );

    return response.data;
  }

  /**
   * Retrieve a payment intent by ID
   *
   * @example
   * ```typescript
   * const intent = await client.paymentIntents.retrieve('pi_1234567890');
   * ```
   */
  async retrieve(id: string): Promise<PaymentIntent> {
    const response = await this.client.request<PaymentIntent>(
      'GET',
      `/payment_intents/${id}`
    );

    return response.data;
  }

  /**
   * Confirm a payment intent with a payment method
   *
   * @example
   * ```typescript
   * const intent = await client.paymentIntents.confirm('pi_1234567890', {
   *   payment_method: {
   *     card: {
   *       token: 'tok_1234567890',
   *     },
   *   },
   * });
   * ```
   */
  async confirm(
    id: string,
    params: ConfirmPaymentIntentParams
  ): Promise<PaymentIntent> {
    const response = await this.client.request<PaymentIntent>(
      'POST',
      `/payment_intents/${id}/confirm`,
      params as unknown as Record<string, unknown>
    );

    return response.data;
  }

  /**
   * Cancel a payment intent
   *
   * @example
   * ```typescript
   * const intent = await client.paymentIntents.cancel('pi_1234567890');
   * ```
   */
  async cancel(
    id: string,
    params?: { cancellation_reason?: string }
  ): Promise<PaymentIntent> {
    const response = await this.client.request<PaymentIntent>(
      'POST',
      `/payment_intents/${id}/cancel`,
      params as unknown as Record<string, unknown>
    );

    return response.data;
  }

  /**
   * Capture a manually captured payment intent
   *
   * @example
   * ```typescript
   * const intent = await client.paymentIntents.capture('pi_1234567890', {
   *   amount_to_capture: 5000,
   * });
   * ```
   */
  async capture(
    id: string,
    params?: { amount_to_capture?: number }
  ): Promise<PaymentIntent> {
    const response = await this.client.request<PaymentIntent>(
      'POST',
      `/payment_intents/${id}/capture`,
      params as unknown as Record<string, unknown>
    );

    return response.data;
  }

  /**
   * List payment intents
   *
   * @example
   * ```typescript
   * const intents = await client.paymentIntents.list({
   *   limit: 10,
   *   customer: 'cus_1234567890',
   * });
   * ```
   */
  async list(params?: {
    customer?: string;
    created?: { gt?: number; gte?: number; lt?: number; lte?: number };
    limit?: number;
    starting_after?: string;
    ending_before?: string;
  }): Promise<{ data: PaymentIntent[]; has_more: boolean }> {
    const queryParams = new URLSearchParams();

    if (params?.customer) queryParams.append('customer', params.customer);
    if (params?.limit) queryParams.append('limit', String(params.limit));
    if (params?.starting_after)
      queryParams.append('starting_after', params.starting_after);
    if (params?.ending_before)
      queryParams.append('ending_before', params.ending_before);

    if (params?.created) {
      if (params.created.gt)
        queryParams.append('created[gt]', String(params.created.gt));
      if (params.created.gte)
        queryParams.append('created[gte]', String(params.created.gte));
      if (params.created.lt)
        queryParams.append('created[lt]', String(params.created.lt));
      if (params.created.lte)
        queryParams.append('created[lte]', String(params.created.lte));
    }

    const query = queryParams.toString();
    const path = `/payment_intents${query ? `?${query}` : ''}`;

    const response = await this.client.request<{
      data: PaymentIntent[];
      has_more: boolean;
    }>('GET', path);

    return response.data;
  }

  /**
   * Process a payment with Apple Pay (iOS only)
   *
   * @example
   * ```typescript
   * const intent = await client.paymentIntents.create({
   *   amount: 5000,
   *   currency: 'usd',
   * });
   *
   * const result = await client.paymentIntents.payWithApplePay(intent.id, {
   *   merchantIdentifier: 'merchant.com.yourcompany',
   *   label: 'Your Store',
   * });
   * ```
   */
  async payWithApplePay(
    paymentIntentId: string,
    options: {
      merchantIdentifier: string;
      label: string;
      amount?: number;
    }
  ): Promise<PaymentIntent> {
    const applePayToken = await this.client.applePay.authorize({
      merchantIdentifier: options.merchantIdentifier,
      label: options.label,
      amount: options.amount,
    });

    return this.confirm(paymentIntentId, {
      payment_method: {
        type: 'card',
        card: {
          token: applePayToken.paymentData,
        },
      },
    });
  }

  /**
   * Process a payment with Google Pay (Android only)
   *
   * @example
   * ```typescript
   * const intent = await client.paymentIntents.create({
   *   amount: 5000,
   *   currency: 'usd',
   * });
   *
   * const result = await client.paymentIntents.payWithGooglePay(intent.id, {
   *   merchantId: 'your-merchant-id',
   *   merchantName: 'Your Store',
   * });
   * ```
   */
  async payWithGooglePay(
    paymentIntentId: string,
    options: {
      merchantId: string;
      merchantName: string;
      amount?: number;
    }
  ): Promise<PaymentIntent> {
    const googlePayToken = await this.client.googlePay.authorize({
      merchantId: options.merchantId,
      merchantName: options.merchantName,
      amount: options.amount,
    });

    return this.confirm(paymentIntentId, {
      payment_method: {
        type: 'card',
        card: {
          token: googlePayToken.paymentMethodData.tokenizationData.token,
        },
      },
    });
  }
}
