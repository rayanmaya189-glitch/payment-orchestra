/**
 * Payment Orchestra Error Handling
 */

export enum ErrorCode {
  // Client errors
  INVALID_REQUEST = 'invalid_request',
  AUTHENTICATION_ERROR = 'authentication_error',
  CARD_ERROR = 'card_error',
  VALIDATION_ERROR = 'validation_error',

  // Network errors
  NETWORK_ERROR = 'network_error',
  TIMEOUT = 'timeout',

  // Server errors
  API_ERROR = 'api_error',
  RATE_LIMITED = 'rate_limited',

  // Native errors
  NATIVE_MODULE_ERROR = 'native_module_error',
  PLATFORM_NOT_SUPPORTED = 'platform_not_supported',

  // Payment errors
  PAYMENT_FAILED = 'payment_failed',
  PAYMENT_CANCELED = 'payment_canceled',
  PAYMENT_DECLINED = 'payment_declined',

  // 3D Secure
  THREE_D_SECURE_FAILED = 'three_d_secure_failed',
  THREE_D_SECURE_REQUIRED = 'three_d_secure_required',

  // Apple Pay / Google Pay
  APPLE_PAY_NOT_AVAILABLE = 'apple_pay_not_available',
  GOOGLE_PAY_NOT_AVAILABLE = 'google_pay_not_available',
  APPLE_PAY_CANCELED = 'apple_pay_canceled',
  GOOGLE_PAY_CANCELED = 'google_pay_canceled',
}

export class PaymentOrchestraError extends Error {
  public readonly code: ErrorCode;
  public readonly statusCode?: number;
  public readonly declineCode?: string;
  public readonly raw?: Record<string, unknown>;

  constructor(
    message: string,
    code: ErrorCode,
    options?: {
      statusCode?: number;
      declineCode?: string;
      raw?: Record<string, unknown>;
    }
  ) {
    super(message);
    this.name = 'PaymentOrchestraError';
    this.code = code;
    this.statusCode = options?.statusCode;
    this.declineCode = options?.declineCode;
    this.raw = options?.raw;

    // Maintains proper stack trace for V8
    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, PaymentOrchestraError);
    }
  }

  /**
   * Check if error is a card error
   */
  isCardError(): boolean {
    return this.code === ErrorCode.CARD_ERROR;
  }

  /**
   * Check if error requires customer action
   */
  requiresAction(): boolean {
    return this.code === ErrorCode.THREE_D_SECURE_REQUIRED;
  }

  /**
   * Check if error is retryable
   */
  isRetryable(): boolean {
    return [
      ErrorCode.NETWORK_ERROR,
      ErrorCode.TIMEOUT,
      ErrorCode.RATE_LIMITED,
    ].includes(this.code);
  }

  /**
   * Get user-friendly error message
   */
  getUserMessage(): string {
    switch (this.code) {
      case ErrorCode.CARD_ERROR:
        return this.message;
      case ErrorCode.AUTHENTICATION_ERROR:
        return 'Please check your API key and try again.';
      case ErrorCode.NETWORK_ERROR:
        return 'Network error. Please check your connection.';
      case ErrorCode.TIMEOUT:
        return 'Request timed out. Please try again.';
      case ErrorCode.RATE_LIMITED:
        return 'Too many requests. Please wait and try again.';
      case ErrorCode.PAYMENT_DECLINED:
        return 'Your card was declined. Please try a different card.';
      case ErrorCode.THREE_D_SECURE_REQUIRED:
        return 'Additional verification required.';
      default:
        return 'An error occurred. Please try again.';
    }
  }
}
