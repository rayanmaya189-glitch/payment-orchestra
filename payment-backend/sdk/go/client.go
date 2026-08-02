// Package paymentorchestra provides a Go SDK for the Payment Orchestra API.
//
// Example:
//
//	client := paymentorchestra.NewClient("pk_test_key", "sk_test_key")
//	payment, err := client.PaymentIntents.Create(&paymentorchestra.CreatePaymentParams{
//	    Amount:   10000,
//	    Currency: "USD",
//	})
package paymentorchestra

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"
)

// Client represents the Payment Orchestra API client.
type Client struct {
	apiKey     string
	secretKey  string
	baseURL    string
	httpClient *http.Client

	PaymentIntents  *PaymentIntentsService
	GatewayProfiles *GatewayProfilesService
	RoutingPolicies *RoutingPoliciesService
	Webhooks        *WebhooksService
}

// Config holds the client configuration.
type Config struct {
	APIKey      string
	SecretKey   string
	Environment string
	BaseURL     string
	Timeout     time.Duration
}

// NewClient creates a new Payment Orchestra client.
func NewClient(apiKey, secretKey string, opts ...Option) *Client {
	cfg := &Config{
		APIKey:      apiKey,
		SecretKey:   secretKey,
		Environment: "sandbox",
		Timeout:     30 * time.Second,
	}

	for _, opt := range opts {
		opt(cfg)
	}

	if cfg.BaseURL == "" {
		if cfg.Environment == "production" {
			cfg.BaseURL = "https://api.paymentorchestra.com"
		} else {
			cfg.BaseURL = "https://sandbox.api.paymentorchestra.com"
		}
	}

	client := &Client{
		apiKey:    cfg.APIKey,
		secretKey: cfg.SecretKey,
		baseURL:   cfg.BaseURL,
		httpClient: &http.Client{
			Timeout: cfg.Timeout,
		},
	}

	client.PaymentIntents = &PaymentIntentsService{client: client}
	client.GatewayProfiles = &GatewayProfilesService{client: client}
	client.RoutingPolicies = &RoutingPoliciesService{client: client}
	client.Webhooks = &WebhooksService{client: client}

	return client
}

// Option is a function that configures the client.
type Option func(*Config)

// WithEnvironment sets the environment.
func WithEnvironment(env string) Option {
	return func(c *Config) {
		c.Environment = env
	}
}

// WithBaseURL sets a custom base URL.
func WithBaseURL(url string) Option {
	return func(c *Config) {
		c.BaseURL = url
	}
}

// WithTimeout sets the HTTP client timeout.
func WithTimeout(timeout time.Duration) Option {
	return func(c *Config) {
		c.Timeout = timeout
	}
}

// APIError represents an API error response.
type APIError struct {
	StatusCode int
	Message    string
	Details    map[string]interface{}
}

func (e *APIError) Error() string {
	return fmt.Sprintf("[%d] %s", e.StatusCode, e.Message)
}

func (c *Client) doRequest(method, path string, body interface{}) ([]byte, error) {
	var reqBody io.Reader

	if body != nil {
		jsonBody, err := json.Marshal(body)
		if err != nil {
			return nil, fmt.Errorf("failed to marshal request body: %w", err)
		}
		reqBody = bytes.NewReader(jsonBody)
	}

	req, err := http.NewRequest(method, c.baseURL+path, reqBody)
	if err != nil {
		return nil, fmt.Errorf("failed to create request: %w", err)
	}

	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+c.apiKey)
	req.Header.Set("X-Secret-Key", c.secretKey)
	req.Header.Set("X-SDK-Version", "1.0.0")
	req.Header.Set("X-SDK-Name", "go")

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("request failed: %w", err)
	}
	defer resp.Body.Close()

	respBody, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("failed to read response body: %w", err)
	}

	if resp.StatusCode >= 400 {
		var errorResp struct {
			Error struct {
				Message string `json:"message"`
			} `json:"error"`
		}
		if err := json.Unmarshal(respBody, &errorResp); err == nil && errorResp.Error.Message != "" {
			return nil, &APIError{
				StatusCode: resp.StatusCode,
				Message:    errorResp.Error.Message,
			}
		}
		return nil, &APIError{
			StatusCode: resp.StatusCode,
			Message:    string(respBody),
		}
	}

	return respBody, nil
}
