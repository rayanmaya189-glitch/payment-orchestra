/**
 * Gateway Profile types for Payment Orchestration Platform SDK
 */

import { GatewayStatus, Metadata } from './common';

export interface CreateGatewayProfileParams {
  connector_id: string;
  merchant_id: string;
  api_key?: string;
  secret_key?: string;
  environment?: 'sandbox' | 'production';
  supported_currencies?: string[];
  supported_card_schemes?: string[];
  metadata?: Metadata;
}

export interface UpdateGatewayProfileParams {
  merchant_id?: string;
  api_key?: string;
  secret_key?: string;
  environment?: 'sandbox' | 'production';
  supported_currencies?: string[];
  supported_card_schemes?: string[];
  status?: GatewayStatus;
  metadata?: Metadata;
}

export interface GatewayProfile {
  id: string;
  object: 'gateway_profile';
  connector_id: string;
  merchant_id: string;
  environment: 'sandbox' | 'production';
  status: GatewayStatus;
  supported_currencies: string[];
  supported_card_schemes: string[];
  success_rate: number;
  avg_latency_ms: number;
  circuit_breaker_state: 'closed' | 'open' | 'half_open';
  metadata?: Metadata;
  created: number;
  updated: number;
}

export interface GatewayProfileListParams {
  limit?: number;
  offset?: number;
  connector_id?: string;
  status?: GatewayStatus;
}

export interface GatewayHealth {
  gateway_id: string;
  is_healthy: boolean;
  success_rate: number;
  avg_latency_ms: number;
  circuit_breaker_state: 'closed' | 'open' | 'half_open';
  last_checked: number;
}
