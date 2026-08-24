/**
 * Payment Intent types for Payment Orchestration Platform SDK
 */

import { Money, PaymentStatus, Metadata } from './common';

export interface CreatePaymentIntentParams {
  amount: number;
  currency: string;
  order_id: string;
  customer_id?: string;
  payment_method?: string;
  capture_method?: 'automatic' | 'manual';
  confirmation_method?: 'automatic' | 'manual';
  metadata?: Metadata;
}

export interface UpdatePaymentIntentParams {
  amount?: number;
  currency?: string;
  order_id?: string;
  customer_id?: string;
  payment_method?: string;
  metadata?: Metadata;
}

export interface PaymentIntent {
  id: string;
  object: 'payment_intent';
  amount: number;
  currency: string;
  status: PaymentStatus;
  order_id: string;
  customer_id?: string;
  payment_method?: string;
  capture_method: 'automatic' | 'manual';
  confirmation_method: 'automatic' | 'manual';
  client_secret: string;
  gateway_profile_id?: string;
  routing_policy_id?: string;
  metadata?: Metadata;
  created: number;
  updated: number;
}

export interface AuthorizePaymentIntentParams {
  payment_method?: string;
  return_url?: string;
}

export interface CapturePaymentIntentParams {
  amount?: number;
}

export interface RefundPaymentIntentParams {
  amount?: number;
  reason?: 'duplicate' | 'fraudulent' | 'requested_by_customer' | 'other';
}

export interface PaymentIntentListParams {
  limit?: number;
  offset?: number;
  status?: PaymentStatus;
  customer_id?: string;
  created_after?: number;
  created_before?: number;
}
