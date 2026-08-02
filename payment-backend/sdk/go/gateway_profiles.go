package paymentorchestra

import (
	"encoding/json"
	"fmt"
)

// GatewayProfilesService handles communication with the Gateway Profiles API.
type GatewayProfilesService struct {
	client *Client
}

// GatewayProfile represents a gateway profile.
type GatewayProfile struct {
	ID                  string   `json:"id"`
	Object              string   `json:"object"`
	ConnectorID         string   `json:"connector_id"`
	DisplayName         string   `json:"display_name"`
	Environment         string   `json:"environment"`
	Status              string   `json:"status"`
	SupportedCurrencies []string `json:"supported_currencies"`
	SupportedCardSchemes []string `json:"supported_card_schemes"`
	SuccessRate         float64  `json:"success_rate,omitempty"`
	AvgLatencyMs        int      `json:"avg_latency_ms,omitempty"`
	LastHealthCheck     string   `json:"last_health_check,omitempty"`
	CreatedAt           string   `json:"created_at"`
}

// CreateGatewayProfileParams holds parameters for creating a gateway profile.
type CreateGatewayProfileParams struct {
	ConnectorID  string            `json:"connector_id"`
	DisplayName  string            `json:"display_name"`
	Environment  string            `json:"environment"`
	Credentials  map[string]string `json:"credentials"`
}

// Create creates a new gateway profile.
func (s *GatewayProfilesService) Create(params *CreateGatewayProfileParams) (*GatewayProfile, error) {
	body, err := s.client.doRequest("POST", "/v1/gateway-profiles", params)
	if err != nil {
		return nil, err
	}

	var result GatewayProfile
	if err := json.Unmarshal(body, &result); err != nil {
		return nil, fmt.Errorf("failed to unmarshal response: %w", err)
	}

	return &result, nil
}

// Get retrieves a gateway profile by ID.
func (s *GatewayProfilesService) Get(id string) (*GatewayProfile, error) {
	body, err := s.client.doRequest("GET", fmt.Sprintf("/v1/gateway-profiles/%s", id), nil)
	if err != nil {
		return nil, err
	}

	var result GatewayProfile
	if err := json.Unmarshal(body, &result); err != nil {
		return nil, fmt.Errorf("failed to unmarshal response: %w", err)
	}

	return &result, nil
}

// List retrieves a list of gateway profiles.
func (s *GatewayProfilesService) List() ([]GatewayProfile, error) {
	body, err := s.client.doRequest("GET", "/v1/gateway-profiles", nil)
	if err != nil {
		return nil, err
	}

	var result []GatewayProfile
	if err := json.Unmarshal(body, &result); err != nil {
		return nil, fmt.Errorf("failed to unmarshal response: %w", err)
	}

	return result, nil
}

// Delete deletes a gateway profile.
func (s *GatewayProfilesService) Delete(id string) error {
	_, err := s.client.doRequest("DELETE", fmt.Sprintf("/v1/gateway-profiles/%s", id), nil)
	return err
}

// Test tests a gateway connection.
func (s *GatewayProfilesService) Test(id string) (map[string]interface{}, error) {
	body, err := s.client.doRequest("POST", fmt.Sprintf("/v1/gateway-profiles/%s/test", id), nil)
	if err != nil {
		return nil, err
	}

	var result map[string]interface{}
	if err := json.Unmarshal(body, &result); err != nil {
		return nil, fmt.Errorf("failed to unmarshal response: %w", err)
	}

	return result, nil
}
