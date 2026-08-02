/**
 * Common types for Payment Orchestration Platform SDK
 */

export interface PaginationParams {
  limit?: number;
  offset?: number;
}

export interface PaginatedResponse<T> {
  data: T[];
  total: number;
  limit: number;
  offset: number;
  has_more: boolean;
}

export interface ApiError {
  code: string;
  message: string;
  details?: Record<string, any>;
}

export interface Money {
  amount_minor_units: number;
  currency: string;
}

export type PaymentStatus = 
  | 'created'
  | 'authorized'
  | 'captured'
  | 'partially_captured'
  | 'voided'
  | 'refunded'
  | 'partially_refunded'
  | 'failed';

export type GatewayStatus = 
  | 'active'
  | 'inactive'
  | 'error'
  | 'testing';

export type RoutingStrategy = 
  | 'priority'
  | 'round_robin'
  | 'weighted_round_robin'
  | 'success_rate'
  | 'cost_based'
  | 'volume_capped';

export interface Metadata {
  [key: string]: string | number | boolean;
}
