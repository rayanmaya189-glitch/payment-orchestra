/**
 * Apple Pay Integration
 *
 * Secure payment processing using Apple Pay on iOS devices.
 */

import { Platform, NativeModules, NativeEventEmitter } from 'react-native';
import { PaymentOrchestraError, ErrorCode } from './errors';
import type { ApplePayConfig, ApplePayToken } from './types';

const { PaymentOrchestraApplePay } = NativeModules;

export class ApplePay {
  private client: import('../client').PaymentOrchestra;
  private emitter?: NativeEventEmitter;

  constructor(client: import('../client').PaymentOrchestra) {
    this.client = client;

    if (Platform.OS === 'ios' && PaymentOrchestraApplePay) {
      this.emitter = new NativeEventEmitter(PaymentOrchestraApplePay);
    }
  }

  /**
   * Check if Apple Pay is available on this device
   */
  async isAvailable(): Promise<boolean> {
    if (Platform.OS !== 'ios') {
      return false;
    }

    if (!PaymentOrchestraApplePay) {
      return false;
    }

    try {
      return await PaymentOrchestraApplePay.canMakePayments();
    } catch {
      return false;
    }
  }

  /**
   * Check if a specific card is supported by Apple Pay
   */
  async canAddCard(cardNetwork: string): Promise<boolean> {
    if (Platform.OS !== 'ios' || !PaymentOrchestraApplePay) {
      return false;
    }

    try {
      return await PaymentOrchestraApplePay.canAddCard(cardNetwork);
    } catch {
      return false;
    }
  }

  /**
   * Present Apple Pay payment sheet and authorize payment
   *
   * @example
   * ```typescript
   * const token = await client.applePay.authorize({
   *   merchantIdentifier: 'merchant.com.yourcompany',
   *   label: 'Your Store',
   *   amount: 50.00,
   *   currency: 'USD',
   * });
   *
   * // Use token to confirm payment
   * await client.paymentIntents.confirm(paymentIntentId, {
   *   payment_method: {
   *     type: 'card',
   *     card: {
   *       token: token.paymentData,
   *     },
   *   },
   * });
   * ```
   */
  async authorize(
    options: ApplePayConfig & {
      amount?: number;
      label?: string;
    }
  ): Promise<ApplePayToken> {
    if (Platform.OS !== 'ios') {
      throw new PaymentOrchestraError(
        'Apple Pay is only available on iOS',
        ErrorCode.PLATFORM_NOT_SUPPORTED
      );
    }

    if (!PaymentOrchestraApplePay) {
      throw new PaymentOrchestraError(
        'Apple Pay native module not available',
        ErrorCode.NATIVE_MODULE_ERROR
      );
    }

    const isAvailable = await this.isAvailable();
    if (!isAvailable) {
      throw new PaymentOrchestraError(
        'Apple Pay is not available on this device',
        ErrorCode.APPLE_PAY_NOT_AVAILABLE
      );
    }

    try {
      const token = await PaymentOrchestraApplePay.authorize({
        merchantIdentifier: options.merchantIdentifier,
        countryCode: options.countryCode,
        currencyCode: options.currencyCode ?? 'USD',
        supportedNetworks: options.supportedNetworks ?? ['visa', 'mastercard', 'amex'],
        merchantCapabilities: options.merchantCapabilities ?? ['3ds'],
        amount: options.amount,
        label: options.label,
      });

      return {
        paymentData: token.paymentData,
        transactionIdentifier: token.transactionIdentifier,
        paymentMethod: {
          displayName: token.paymentMethod.displayName,
          network: token.paymentMethod.network,
          type: token.paymentMethod.type,
        },
      };
    } catch (error) {
      if (error instanceof PaymentOrchestraError) {
        throw error;
      }

      // User canceled
      if (String(error).includes('canceled')) {
        throw new PaymentOrchestraError(
          'Payment was canceled',
          ErrorCode.APPLE_PAY_CANCELED
        );
      }

      throw new PaymentOrchestraError(
        `Apple Pay authorization failed: ${String(error)}`,
        ErrorCode.APPLE_PAY_NOT_AVAILABLE
      );
    }
  }

  /**
   * Add a card to Apple Pay
   */
  async addCard(
    options: ApplePayConfig
  ): Promise<{ success: boolean; token?: ApplePayToken }> {
    if (Platform.OS !== 'ios' || !PaymentOrchestraApplePay) {
      return { success: false };
    }

    try {
      const result = await PaymentOrchestraApplePay.addCard({
        merchantIdentifier: options.merchantIdentifier,
        countryCode: options.countryCode,
        supportedNetworks: options.supportedNetworks ?? ['visa', 'mastercard'],
        merchantCapabilities: options.merchantCapabilities ?? ['3ds'],
      });

      return {
        success: true,
        token: result.token,
      };
    } catch {
      return { success: false };
    }
  }

  /**
   * Listen for Apple Pay payment events
   */
  onPaymentAuthorized(callback: (token: ApplePayToken) => void): () => void {
    if (!this.emitter) {
      return () => {};
    }

    const subscription = this.emitter.addListener(
      'onPaymentAuthorized',
      (token: ApplePayToken) => {
        callback(token);
      }
    );

    return () => {
      subscription.remove();
    };
  }

  /**
   * Listen for Apple Pay payment canceled events
   */
  onPaymentCanceled(callback: () => void): () => void {
    if (!this.emitter) {
      return () => {};
    }

    const subscription = this.emitter.addListener('onPaymentCanceled', () => {
      callback();
    });

    return () => {
      subscription.remove();
    };
  }

  /**
   * Listen for Apple Pay errors
   */
  onError(callback: (error: Error) => void): () => void {
    if (!this.emitter) {
      return () => {};
    }

    const subscription = this.emitter.addListener(
      'onPaymentError',
      (error: { message: string }) => {
        callback(new Error(error.message));
      }
    );

    return () => {
      subscription.remove();
    };
  }
}
