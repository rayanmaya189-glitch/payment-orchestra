package paymentorchestra

import (
	"encoding/json"
	"fmt"
)

// RoutingPoliciesService handles communication with the Routing Policies API.
type RoutingPoliciesService struct {
	client *Client
}

// RoutingPolicy represents a routing policy.
type RoutingPolicy struct {
	ID                string            `json:"id"`
	Object            string            `json:"object"`
	Name              string            `json:"name"`
	Status            string            `json:"status"`
	Rules             []RoutingRule     `json:"rules"`
	RotationStrategy  string            `json:"rotation_strategy"`
	FailoverConfig    FailoverConfig    `json:"failover_config"`
	CreatedAt         string            `json:"created_at"`
	ActivatedAt       string            `json:"activated_at,omitempty"`
}

// RoutingRule represents a routing rule.
type RoutingRule struct {
	ID                string           `json:"id"`
	GatewayProfileID  string           `json:"gateway_profile_id"`
	Priority          int              `json:"priority"`
	Condition         RoutingCondition `json:"condition"`
}

// RoutingCondition represents routing conditions.
type RoutingCondition struct {
	CardSchemes []string `json:"card_schemes,omitempty"`
	Currencies  []string `json:"currencies,omitempty"`
	MinAmount   *int64   `json:"min_amount,omitempty"`
	MaxAmount   *int64   `json:"max_amount,omitempty"`
}

// FailoverConfig represents failover configuration.
type FailoverConfig struct {
	MaxHops         int `json:"max_hops"`
	LatencyBudgetMs int `json:"latency_budget_ms"`
}

// CreateRoutingPolicyParams holds parameters for creating a routing policy.
type CreateRoutingPolicyParams struct {
	Name             string        `json:"name"`
	Rules            []RoutingRule `json:"rules"`
	RotationStrategy string        `json:"rotation_strategy"`
	FailoverConfig   *FailoverConfig `json:"failover_config,omitempty"`
}

// Create creates a new routing policy.
func (s *RoutingPoliciesService) Create(params *CreateRoutingPolicyParams) (*RoutingPolicy, error) {
	body, err := s.client.doRequest("POST", "/v1/routing-policies", params)
	if err != nil {
		return nil, err
	}

	var result RoutingPolicy
	if err := json.Unmarshal(body, &result); err != nil {
		return nil, fmt.Errorf("failed to unmarshal response: %w", err)
	}

	return &result, nil
}

// Get retrieves a routing policy by ID.
func (s *RoutingPoliciesService) Get(id string) (*RoutingPolicy, error) {
	body, err := s.client.doRequest("GET", fmt.Sprintf("/v1/routing-policies/%s", id), nil)
	if err != nil {
		return nil, err
	}

	var result RoutingPolicy
	if err := json.Unmarshal(body, &result); err != nil {
		return nil, fmt.Errorf("failed to unmarshal response: %w", err)
	}

	return &result, nil
}

// List retrieves a list of routing policies.
func (s *RoutingPoliciesService) List() ([]RoutingPolicy, error) {
	body, err := s.client.doRequest("GET", "/v1/routing-policies", nil)
	if err != nil {
		return nil, err
	}

	var result []RoutingPolicy
	if err := json.Unmarshal(body, &result); err != nil {
		return nil, fmt.Errorf("failed to unmarshal response: %w", err)
	}

	return result, nil
}

// Activate activates a routing policy.
func (s *RoutingPoliciesService) Activate(id string) (*RoutingPolicy, error) {
	body, err := s.client.doRequest("POST", fmt.Sprintf("/v1/routing-policies/%s/activate", id), nil)
	if err != nil {
		return nil, err
	}

	var result RoutingPolicy
	if err := json.Unmarshal(body, &result); err != nil {
		return nil, fmt.Errorf("failed to unmarshal response: %w", err)
	}

	return &result, nil
}

// Deactivate deactivates a routing policy.
func (s *RoutingPoliciesService) Deactivate(id string) (*RoutingPolicy, error) {
	body, err := s.client.doRequest("POST", fmt.Sprintf("/v1/routing-policies/%s/deactivate", id), nil)
	if err != nil {
		return nil, err
	}

	var result RoutingPolicy
	if err := json.Unmarshal(body, &result); err != nil {
		return nil, fmt.Errorf("failed to unmarshal response: %w", err)
	}

	return &result, nil
}
