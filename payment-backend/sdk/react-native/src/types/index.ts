/**
 * Payment Orchestra React Native SDK - Type Definitions
 */

// Environment
export type Environment = 'sandbox' | 'production';

// Client Configuration
export interface PaymentOrchestraConfig {
  apiKey: string;
  environment?: Environment;
  baseUrl?: string;
  timeout?: number;
  debug?: boolean;
}

// API Response
export interface ApiResponse<T> {
  data: T;
  status: 'success' | 'error';
  error?: ApiError;
}

export interface ApiError {
  code: string;
  message: string;
  type: string;
  decline_code?: string;
}

// Payment Intent Types
export interface PaymentIntent {
  id: string;
  object: 'payment_intent';
  amount: number;
  currency: string;
  status: PaymentIntentStatus;
  payment_method_types: string[];
  payment_method?: PaymentMethod;
  customer?: string;
  metadata: Record<string, string>;
  client_secret: string;
  created: number;
  livemode: boolean;
  next_action?: NextAction;
  latest_charge?: string;
  capture_method: 'automatic' | 'manual';
  confirmation_method: 'automatic' | 'manual';
}

export type PaymentIntentStatus =
  | 'requires_payment_method'
  | 'requires_confirmation'
  | 'requires_action'
  | 'requires_capture'
  | 'processing'
  | 'succeeded'
  | 'canceled'
  | 'requires_failed_request';

export interface NextAction {
  type: string;
  redirect_to_url?: {
    return_url: string;
    url: string;
  };
  use_stripe_sdk?: Record<string, unknown>;
}

export interface CreatePaymentIntentParams {
  amount: number;
  currency: string;
  payment_method_types?: string[];
  customer?: string;
  metadata?: Record<string, string>;
  capture_method?: 'automatic' | 'manual';
  confirmation_method?: 'automatic' | 'manual';
  confirm?: boolean;
}

export interface ConfirmPaymentIntentParams {
  payment_method: PaymentMethodParams;
  return_url?: string;
  receipt_email?: string;
  metadata?: Record<string, string>;
}

// Payment Method Types
export interface PaymentMethod {
  id: string;
  object: 'payment_method';
  type: PaymentMethodType;
  card?: CardDetails;
  billing_details?: BillingDetails;
  created: number;
  livemode: boolean;
  metadata: Record<string, string>;
}

export type PaymentMethodType = 'card' | 'bank_account' | 'wallet' | 'upi';

export interface CardDetails {
  brand: string;
  country: string;
  exp_month: number;
  exp_year: number;
  fingerprint: string;
  funding: string;
  last4: string;
  three_d_secure_usage?: {
    supported: boolean;
  };
  wallet?: {
    type: string;
  };
}

export interface BillingDetails {
  address?: Address;
  email?: string;
  name?: string;
  phone?: string;
}

export interface Address {
  city?: string;
  country?: string;
  line1?: string;
  line2?: string;
  postal_code?: string;
  state?: string;
}

export interface PaymentMethodParams {
  type: string;
  card?: {
    number?: string;
    exp_month?: number;
    exp_year?: number;
    cvc?: string;
    token?: string;
    three_d_secure?: boolean;
  };
  billing_details?: BillingDetails;
}

// Customer Types
export interface Customer {
  id: string;
  object: 'customer';
  email?: string;
  name?: string;
  phone?: string;
  address?: Address;
  metadata: Record<string, string>;
  created: number;
  livemode: boolean;
  default_payment_method?: string;
}

export interface CreateCustomerParams {
  email?: string;
  name?: string;
  phone?: string;
  address?: Address;
  metadata?: Record<string, string>;
  payment_method?: string;
}

// Refund Types
export interface Refund {
  id: string;
  object: 'refund';
  amount: number;
  currency: string;
  payment_intent: string;
  status: RefundStatus;
  reason?: RefundReason;
  receipt_number?: string;
  metadata: Record<string, string>;
  created: number;
}

export type RefundStatus = 'pending' | 'succeeded' | 'failed' | 'canceled';
export type RefundReason =
  | 'duplicate'
  | 'fraudulent'
  | 'requested_by_customer'
  | 'expired_uncaptured';

export interface CreateRefundParams {
  amount?: number;
  reason?: RefundReason;
  metadata?: Record<string, string>;
}

// Apple Pay Types
export interface ApplePayConfig {
  merchantId: string;
  countryCode: string;
  currencyCode: string;
  supportedNetworks: string[];
  merchantCapabilities: string[];
}

export interface ApplePayToken {
  paymentData: string;
  transactionIdentifier: string;
  paymentMethod: {
    displayName: string;
    network: string;
    type: string;
  };
}

// Google Pay Types
export interface GooglePayConfig {
  merchantId: string;
  countryCode: string;
  environment: 'TEST' | 'PRODUCTION';
  allowedPaymentMethods: string[];
  merchantName: string;
}

export interface GooglePayToken {
  paymentMethodData: {
    tokenizationData: {
      token: string;
      type: string;
    };
    info: {
      cardNetwork: string;
      cardDetails: string;
    };
    type: string;
  };
}

// Webhook Types
export interface WebhookEvent {
  id: string;
  type: string;
  data: {
    object: Record<string, unknown>;
  };
  created: number;
}

// Device Info
export interface DeviceInfo {
  platform: string;
  version: string;
  model: string;
  brand?: string;
  deviceId?: string;
  isEmulator: boolean;
  isJailbroken: boolean;
}

// SCA (Strong Customer Authentication)
export interface SCAConfig {
  enabled: boolean;
  threshold?: number;
  captureMethod?: 'automatic' | 'manual';
}

// 3D Secure
export interface ThreeDSecureConfig {
  enabled: boolean;
  version?: '1.0' | '2.0' | '2.1' | '2.2';
}

// Fraud Detection
export interface FraudDetectionConfig {
  enabled: boolean;
  deviceId?: string;
  ipAddress?: string;
  email?: string;
}

// UI Components Props
export interface CardFormProps {
  onCardTokenized: (token: string) => void;
  onError: (error: Error) => void;
  style?: Record<string, unknown>;
  placeholder?: {
    number?: string;
    expiry?: string;
    cvc?: string;
    name?: string;
  };
  appearance?: {
    colors?: Record<string, string>;
    fonts?: Record<string, string>;
  };
}

export interface PaymentSheetProps {
  clientSecret: string;
  onComplete: (result: PaymentSheetResult) => void;
  onCancel?: () => void;
  style?: Record<string, unknown>;
  appearance?: {
    colors?: Record<string, string>;
    fonts?: Record<string, string>;
  };
}

export interface PaymentSheetResult {
  status: 'succeeded' | 'failed' | 'canceled';
  paymentIntent?: PaymentIntent;
  error?: Error;
}

// Hooks Types
export interface UsePaymentOrchestraResult {
  client: import('../client').PaymentOrchestra | null;
  isLoading: boolean;
  error: Error | null;
}

export interface UsePaymentIntentResult {
  paymentIntent: PaymentIntent | null;
  isLoading: boolean;
  error: Error | null;
  confirm: (params: ConfirmPaymentIntentParams) => Promise<PaymentIntent>;
  cancel: () => Promise<void>;
}
