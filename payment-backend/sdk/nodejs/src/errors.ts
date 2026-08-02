/**
 * Error classes for Payment Orchestration Platform SDK
 */

export class PaymentOrchestraError extends Error {
  public readonly statusCode: number;
  public readonly code: string;
  public readonly details?: any;

  constructor(statusCode: number, message: string, details?: any) {
    super(message);
    this.name = 'PaymentOrchestraError';
    this.statusCode = statusCode;
    this.code = this.extractCode(details);
    this.details = details;
  }

  private extractCode(details: any): string {
    if (details?.error?.code) {
      return details.error.code;
    }
    if (details?.code) {
      return details.code;
    }
    return 'unknown_error';
  }
}

export class AuthenticationError extends PaymentOrchestraError {
  constructor(message: string = 'Authentication failed') {
    super(401, message);
    this.name = 'AuthenticationError';
  }
}

export class RateLimitError extends PaymentOrchestraError {
  public readonly retryAfter?: number;

  constructor(message: string = 'Rate limit exceeded', retryAfter?: number) {
    super(429, message);
    this.name = 'RateLimitError';
    this.retryAfter = retryAfter;
  }
}

export class ValidationError extends PaymentOrchestraError {
  constructor(message: string = 'Validation error', details?: any) {
    super(400, message, details);
    this.name = 'ValidationError';
  }
}

export class NotFoundError extends PaymentOrchestraError {
  constructor(message: string = 'Resource not found') {
    super(404, message);
    this.name = 'NotFoundError';
  }
}

export class GatewayError extends PaymentOrchestraError {
  constructor(message: string = 'Gateway error', details?: any) {
    super(502, message, details);
    this.name = 'GatewayError';
  }
}

export class NetworkError extends PaymentOrchestraError {
  constructor(message: string = 'Network error') {
    super(503, message);
    this.name = 'NetworkError';
  }
}
