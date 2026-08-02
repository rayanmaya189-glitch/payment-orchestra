// Payment Intent Types
export type PaymentStatus = 
  | 'created'
  | 'authorizing'
  | 'authorized'
  | 'capturing'
  | 'captured'
  | 'partially_captured'
  | 'voiding'
  | 'voided'
  | 'refunding'
  | 'refunded'
  | 'partially_refunded'
  | 'failed'
  | 'failed_all_routes';

export interface PaymentIntent {
  id: string;
  object: 'payment_intent';
  amount: number;
  currency: string;
  status: PaymentStatus;
  order_id?: string;
  customer_id?: string;
  payment_method_token_id?: string;
  routing_policy_id?: string;
  gateway_profile_id?: string;
  risk_score?: number;
  risk_level?: string;
  authorized_amount: number;
  captured_amount: number;
  refunded_amount: number;
  metadata?: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

// Gateway Profile Types
export type GatewayStatus = 'active' | 'inactive' | 'error' | 'testing';

export interface GatewayProfile {
  id: string;
  object: 'gateway_profile';
  connector_id: string;
  display_name: string;
  environment: 'sandbox' | 'production';
  status: GatewayStatus;
  supported_currencies: string[];
  supported_card_schemes: string[];
  last_health_check?: string;
  success_rate?: number;
  avg_latency_ms?: number;
  created_at: string;
}

// Routing Policy Types
export type RotationStrategy = 
  | 'priority'
  | 'round_robin'
  | 'weighted_round_robin'
  | 'cost_based'
  | 'success_rate_based'
  | 'volume_capped';

export interface RoutingCondition {
  card_schemes?: string[];
  currencies?: string[];
  min_amount?: number;
  max_amount?: number;
}

export interface RoutingRule {
  id: string;
  gateway_profile_id: string;
  priority: number;
  condition: RoutingCondition;
}

export interface RoutingPolicy {
  id: string;
  object: 'routing_policy';
  name: string;
  status: 'active' | 'inactive';
  rules: RoutingRule[];
  rotation_strategy: RotationStrategy;
  failover_config: {
    max_hops: number;
    latency_budget_ms: number;
  };
  created_at: string;
  activated_at?: string;
}

// Webhook Types
export type WebhookEvent = 
  | 'payment_intent.created'
  | 'payment_intent.authorized'
  | 'payment_intent.captured'
  | 'payment_intent.failed'
  | 'payment_intent.refunded'
  | 'gateway.health_changed';

export interface WebhookEndpoint {
  id: string;
  url: string;
  events: WebhookEvent[];
  status: 'active' | 'inactive';
  created_at: string;
}

// API Key Types
export interface ApiKey {
  id: string;
  name: string;
  key_prefix: string;
  scopes: string[];
  environment: 'sandbox' | 'production';
  last_used_at?: string;
  expires_at?: string;
  created_at: string;
}

// Analytics Types
export interface TransactionSummary {
  total_count: number;
  total_amount: number;
  success_rate: number;
  avg_latency_ms: number;
  period: string;
}

export interface DailyMetric {
  date: string;
  count: number;
  amount: number;
  success_rate: number;
}

export interface GatewayPerformance {
  gateway_id: string;
  name: string;
  success_rate: number;
  avg_latency_ms: number;
  transaction_count: number;
  revenue_share: number;
}

// Dashboard Types
export interface DashboardMetrics {
  total_transactions: number;
  total_volume: number;
  success_rate: number;
  avg_latency_ms: number;
  active_gateways: number;
  routing_savings: number;
}

// Connector Types
export interface Connector {
  id: string;
  name: string;
  description: string;
  category: 'card_processing' | 'wallet' | 'bank_transfer' | 'bnpl' | 'alternative';
  supported_currencies: string[];
  supported_countries: string[];
  features: string[];
  requires_credentials: CredentialField[];
}

export interface CredentialField {
  name: string;
  label: string;
  type: 'text' | 'password' | 'select';
  required: boolean;
  help_text?: string;
  options?: { value: string; label: string }[];
}

// User Types
export interface User {
  id: string;
  email: string;
  name: string;
  role: 'owner' | 'admin' | 'developer' | 'viewer';
  avatar_url?: string;
}

export interface Organization {
  id: string;
  name: string;
  tier: 'starter' | 'growth' | 'enterprise';
  status: 'active' | 'trial' | 'suspended';
  trial_ends_at?: string;
}
