package paymentorchestra

import (
	"encoding/json"
	"fmt"
)

// WebhooksService handles communication with the Webhooks API.
type WebhooksService struct {
	client *Client
}

// WebhookEndpoint represents a webhook endpoint.
type WebhookEndpoint struct {
	ID        string   `json:"id"`
	URL       string   `json:"url"`
	Events    []string `json:"events"`
	Status    string   `json:"status"`
	CreatedAt string   `json:"created_at"`
}

// CreateWebhookParams holds parameters for creating a webhook endpoint.
type CreateWebhookParams struct {
	URL    string   `json:"url"`
	Events []string `json:"events"`
}

// Create creates a new webhook endpoint.
func (s *WebhooksService) Create(params *CreateWebhookParams) (*WebhookEndpoint, error) {
	body, err := s.client.doRequest("POST", "/v1/webhooks", params)
	if err != nil {
		return nil, err
	}

	var result WebhookEndpoint
	if err := json.Unmarshal(body, &result); err != nil {
		return nil, fmt.Errorf("failed to unmarshal response: %w", err)
	}

	return &result, nil
}

// Get retrieves a webhook endpoint by ID.
func (s *WebhooksService) Get(id string) (*WebhookEndpoint, error) {
	body, err := s.client.doRequest("GET", fmt.Sprintf("/v1/webhooks/%s", id), nil)
	if err != nil {
		return nil, err
	}

	var result WebhookEndpoint
	if err := json.Unmarshal(body, &result); err != nil {
		return nil, fmt.Errorf("failed to unmarshal response: %w", err)
	}

	return &result, nil
}

// List retrieves a list of webhook endpoints.
func (s *WebhooksService) List() ([]WebhookEndpoint, error) {
	body, err := s.client.doRequest("GET", "/v1/webhooks", nil)
	if err != nil {
		return nil, err
	}

	var result []WebhookEndpoint
	if err := json.Unmarshal(body, &result); err != nil {
		return nil, fmt.Errorf("failed to unmarshal response: %w", err)
	}

	return result, nil
}

// Delete deletes a webhook endpoint.
func (s *WebhooksService) Delete(id string) error {
	_, err := s.client.doRequest("DELETE", fmt.Sprintf("/v1/webhooks/%s", id), nil)
	return err
}

// Test sends a test webhook event.
func (s *WebhooksService) Test(id string) (map[string]interface{}, error) {
	body, err := s.client.doRequest("POST", fmt.Sprintf("/v1/webhooks/%s/test", id), nil)
	if err != nil {
		return nil, err
	}

	var result map[string]interface{}
	if err := json.Unmarshal(body, &result); err != nil {
		return nil, fmt.Errorf("failed to unmarshal response: %w", err)
	}

	return result, nil
}
