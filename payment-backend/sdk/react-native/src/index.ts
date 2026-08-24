/**
 * Payment Orchestra React Native SDK
 *
 * A complete mobile SDK for the Payment Orchestra payment orchestration platform.
 * Supports iOS and Android with native performance.
 *
 * @example
 * ```typescript
 * import { PaymentOrchestra } from '@payment-orchestra/react-native-sdk';
 *
 * const client = new PaymentOrchestra({
 *   apiKey: 'pk_test_...',
 *   environment: 'sandbox',
 * });
 *
 * // Create a payment intent
 * const intent = await client.paymentIntents.create({
 *   amount: 5000,
 *   currency: 'usd',
 *   payment_method_types: ['card'],
 * });
 *
 * // Process payment with card
 * const result = await client.paymentIntents.confirm(intent.id, {
 *   payment_method: {
 *     card: {
 *       number: '4242424242424242',
 *       exp_month: 12,
 *       exp_year: 2025,
 *       cvc: '123',
 *     },
 *   },
 * });
 * ```
 */

// Core client
export { PaymentOrchestra } from './client';
export { PaymentOrchestraError, ErrorCode } from './errors';

// Resources
export { PaymentIntents } from './resources/payment-intents';
export { PaymentMethods } from './resources/payment-methods';
export { Customers } from './resources/customers';
export { Refunds } from './resources/refunds';

// Apple Pay / Google Pay
export { ApplePay } from './apple-pay';
export { GooglePay } from './google-pay';

// Types
export * from './types';

// Hooks
export { usePaymentOrchestra } from './hooks/use-payment-orchestra';
export { usePaymentIntent } from './hooks/use-payment-intent';

// Components
export { CardForm } from './components/card-form';
export { PaymentSheet } from './components/payment-sheet';
