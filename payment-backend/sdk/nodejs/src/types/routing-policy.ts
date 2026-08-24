/**
 * Routing Policy types for Payment Orchestration Platform SDK
 */

import { RoutingStrategy, Metadata } from './common';

export interface RoutingCondition {
  card_schemes?: string[];
  currencies?: string[];
  min_amount?: number;
  max_amount?: number;
}

export interface RoutingRule {
  condition: RoutingCondition;
  gateway_profile_id: string;
  priority: number;
  weight?: number;
}

export interface FailoverConfig {
  max_hops?: number;
  latency_budget_ms?: number;
}

export interface CreateRoutingPolicyParams {
  name: string;
  rules: RoutingRule[];
  rotation_strategy?: RoutingStrategy;
  failover_config?: FailoverConfig;
  metadata?: Metadata;
}

export interface UpdateRoutingPolicyParams {
  name?: string;
  rules?: RoutingRule[];
  rotation_strategy?: RoutingStrategy;
  failover_config?: FailoverConfig;
  status?: 'active' | 'inactive';
  metadata?: Metadata;
}

export interface RoutingPolicy {
  id: string;
  object: 'routing_policy';
  name: string;
  version: number;
  status: 'active' | 'inactive';
  rules: RoutingRule[];
  rotation_strategy: RoutingStrategy;
  failover_config: FailoverConfig;
  metadata?: Metadata;
  created: number;
  updated: number;
}

export interface RoutingPolicyListParams {
  limit?: number;
  offset?: number;
  status?: 'active' | 'inactive';
}

export interface SelectRouteParams {
  card_scheme: string;
  currency: string;
  amount: number;
  attempted_gateways?: string[];
}

export interface SelectedRoute {
  gateway_profile_id: string;
  routing_reason: string;
  success_rate?: number;
}
