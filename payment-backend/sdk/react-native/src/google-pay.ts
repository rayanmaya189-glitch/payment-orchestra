/**
 * Google Pay Integration
 *
 * Secure payment processing using Google Pay on Android devices.
 */

import { Platform, NativeModules, NativeEventEmitter } from 'react-native';
import { PaymentOrchestraError, ErrorCode } from './errors';
import type { GooglePayConfig, GooglePayToken } from './types';

const { PaymentOrchestraGooglePay } = NativeModules;

export class GooglePay {
  private client: import('../client').PaymentOrchestra;
  private emitter?: NativeEventEmitter;

  constructor(client: import('../client').PaymentOrchestra) {
    this.client = client;

    if (Platform.OS === 'android' && PaymentOrchestraGooglePay) {
      this.emitter = new NativeEventEmitter(PaymentOrchestraGooglePay);
    }
  }

  /**
   * Check if Google Pay is available on this device
   */
  async isAvailable(): Promise<boolean> {
    if (Platform.OS !== 'android') {
      return false;
    }

    if (!PaymentOrchestraGooglePay) {
      return false;
    }

    try {
      return await PaymentOrchestraGooglePay.canMakePayments();
    } catch {
      return false;
    }
  }

  /**
   * Check if Google Pay is ready to process payments
   * (Includes card network support check)
   */
  async isReadyToPay(allowedPaymentMethods: string[]): Promise<boolean> {
    if (Platform.OS !== 'android' || !PaymentOrchestraGooglePay) {
      return false;
    }

    try {
      return await PaymentOrchestraGooglePay.isReadyToPay(allowedPaymentMethods);
    } catch {
      return false;
    }
  }

  /**
   * Present Google Pay payment sheet and authorize payment
   *
   * @example
   * ```typescript
   * const token = await client.googlePay.authorize({
   *   merchantId: 'your-merchant-id',
   *   merchantName: 'Your Store',
   *   amount: 50.00,
   *   currency: 'USD',
   *   environment: 'TEST',
   * });
   *
   * // Use token to confirm payment
   * await client.paymentIntents.confirm(paymentIntentId, {
   *   payment_method: {
   *     type: 'card',
   *     card: {
   *       token: token.paymentMethodData.tokenizationData.token,
   *     },
   *   },
   * });
   * ```
   */
  async authorize(
    options: GooglePayConfig & {
      amount?: number;
      currency?: string;
    }
  ): Promise<GooglePayToken> {
    if (Platform.OS !== 'android') {
      throw new PaymentOrchestraError(
        'Google Pay is only available on Android',
        ErrorCode.PLATFORM_NOT_SUPPORTED
      );
    }

    if (!PaymentOrchestraGooglePay) {
      throw new PaymentOrchestraError(
        'Google Pay native module not available',
        ErrorCode.NATIVE_MODULE_ERROR
      );
    }

    const isAvailable = await this.isAvailable();
    if (!isAvailable) {
      throw new PaymentOrchestraError(
        'Google Pay is not available on this device',
        ErrorCode.GOOGLE_PAY_NOT_AVAILABLE
      );
    }

    try {
      const token = await PaymentOrchestraGooglePay.authorize({
        merchantId: options.merchantId,
        merchantName: options.merchantName,
        countryCode: options.countryCode,
        environment: options.environment ?? 'TEST',
        allowedPaymentMethods: options.allowedPaymentMethods ?? ['CARD'],
        amount: options.amount,
        currency: options.currency ?? 'USD',
      });

      return {
        paymentMethodData: {
          tokenizationData: {
            token: token.paymentMethodData.tokenizationData.token,
            type: token.paymentMethodData.tokenizationData.type,
          },
          info: {
            cardNetwork: token.paymentMethodData.info.cardNetwork,
            cardDetails: token.paymentMethodData.info.cardDetails,
          },
          type: token.paymentMethodData.type,
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
          ErrorCode.GOOGLE_PAY_CANCELED
        );
      }

      throw new PaymentOrchestraError(
        `Google Pay authorization failed: ${String(error)}`,
        ErrorCode.GOOGLE_PAY_NOT_AVAILABLE
      );
    }
  }

  /**
   * Create a Google Pay payment request
   * Used for more advanced payment flows
   */
  async createPaymentRequest(params: {
    allowedPaymentMethods: Array<{
      type: string;
      parameters: {
        allowedCardNetworks: string[];
        allowedAuthMethods: string[];
      };
      tokenizationSpecification: {
        type: string;
        parameters: {
          gateway: string;
          gatewayMerchantId: string;
        };
      };
    }>;
    transactionInfo: {
      totalPriceStatus: string;
      totalPrice: string;
      currencyCode: string;
      countryCode: string;
    };
    merchantInfo: {
      merchantName: string;
      merchantId?: string;
    };
  }): Promise<GooglePayToken> {
    if (Platform.OS !== 'android' || !PaymentOrchestraGooglePay) {
      throw new PaymentOrchestraError(
        'Google Pay is only available on Android',
        ErrorCode.PLATFORM_NOT_SUPPORTED
      );
    }

    try {
      return await PaymentOrchestraGooglePay.createPaymentRequest(params);
    } catch (error) {
      throw new PaymentOrchestraError(
        `Google Pay payment request failed: ${String(error)}`,
        ErrorCode.GOOGLE_PAY_NOT_AVAILABLE
      );
    }
  }

  /**
   * Listen for Google Pay payment success events
   */
  onPaymentSuccess(callback: (token: GooglePayToken) => void): () => void {
    if (!this.emitter) {
      return () => {};
    }

    const subscription = this.emitter.addListener(
      'onPaymentSuccess',
      (token: GooglePayToken) => {
        callback(token);
      }
    );

    return () => {
      subscription.remove();
    };
  }

  /**
   * Listen for Google Pay payment canceled events
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
   * Listen for Google Pay errors
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
