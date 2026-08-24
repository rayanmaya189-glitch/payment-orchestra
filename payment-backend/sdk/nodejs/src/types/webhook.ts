/**
 * Webhook types for Payment Orchestration Platform SDK
 */

import { Metadata } from './common';

export interface CreateWebhookParams {
  url: string;
  events: string[];
  secret?: string;
  metadata?: Metadata;
}

export interface UpdateWebhookParams {
  url?: string;
  events?: string[];
  secret?: string;
  status?: 'active' | 'inactive';
  metadata?: Metadata;
}

export interface Webhook {
  id: string;
  object: 'webhook';
  url: string;
  events: string[];
  status: 'active' | 'inactive';
  secret: string;
  metadata?: Metadata;
  created: number;
  updated: number;
}

export interface WebhookListParams {
  limit?: number;
  offset?: number;
  status?: 'active' | 'inactive';
}

export interface WebhookEvent {
  id: string;
  type: string;
  created: number;
  data: {
    object: any;
  };
  api_version?: string;
  livemode?: boolean;
}

export interface WebhookDelivery {
  id: string;
  webhook_id: string;
  event_id: string;
  status: 'success' | 'failed' | 'pending';
  response_code?: number;
  response_body?: string;
  error_message?: string;
  created: number;
}

export type WebhookEventType =
  | 'payment_intent.created'
  | 'payment_intent.authorized'
  | 'payment_intent.captured'
  | 'payment_intent.failed'
  | 'payment_intent.refunded'
  | 'gateway.health_changed'
  | 'gateway.circuit_breaker_opened'
  | 'routing.policy.activated'
  | 'routing.policy.deactivated';
