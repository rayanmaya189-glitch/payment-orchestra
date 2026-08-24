package paymentorchestra

import (
	"encoding/json"
	"fmt"
)

// PaymentIntentsService handles communication with the Payment Intents API.
type PaymentIntentsService struct {
	client *Client
}

// PaymentIntent represents a payment intent.
type PaymentIntent struct {
	ID                string                 `json:"id"`
	Object            string                 `json:"object"`
	Amount            int64                  `json:"amount"`
	Currency          string                 `json:"currency"`
	Status            string                 `json:"status"`
	OrderID           string                 `json:"order_id,omitempty"`
	CustomerID        string                 `json:"customer_id,omitempty"`
	AuthorizedAmount  int64                  `json:"authorized_amount"`
	CapturedAmount    int64                  `json:"captured_amount"`
	RefundedAmount    int64                  `json:"refunded_amount"`
	RiskScore         float64                `json:"risk_score,omitempty"`
	Metadata          map[string]interface{} `json:"metadata,omitempty"`
	CreatedAt         string                 `json:"created_at"`
	UpdatedAt         string                 `json:"updated_at"`
}

// CreatePaymentParams holds parameters for creating a payment intent.
type CreatePaymentParams struct {
	Amount     int64                  `json:"amount"`
	Currency   string                 `json:"currency"`
	OrderID    string                 `json:"order_id,omitempty"`
	CustomerID string                 `json:"customer_id,omitempty"`
	Metadata   map[string]interface{} `json:"metadata,omitempty"`
}

// ListPaymentParams holds parameters for listing payment intents.
type ListPaymentParams struct {
	Limit     int    `json:"limit,omitempty"`
	Offset    int    `json:"offset,omitempty"`
	Status    string `json:"status,omitempty"`
	StartDate string `json:"start_date,omitempty"`
	EndDate   string `json:"end_date,omitempty"`
}

// PaymentIntentList represents a list of payment intents.
type PaymentIntentList struct {
	Items []PaymentIntent `json:"items"`
	Total int             `json:"total"`
}

// Create creates a new payment intent.
func (s *PaymentIntentsService) Create(params *CreatePaymentParams) (*PaymentIntent, error) {
	body, err := s.client.doRequest("POST", "/v1/payment-intents", params)
	if err != nil {
		return nil, err
	}

	var result PaymentIntent
	if err := json.Unmarshal(body, &result); err != nil {
		return nil, fmt.Errorf("failed to unmarshal response: %w", err)
	}

	return &result, nil
}

// Get retrieves a payment intent by ID.
func (s *PaymentIntentsService) Get(id string) (*PaymentIntent, error) {
	body, err := s.client.doRequest("GET", fmt.Sprintf("/v1/payment-intents/%s", id), nil)
	if err != nil {
		return nil, err
	}

	var result PaymentIntent
	if err := json.Unmarshal(body, &result); err != nil {
		return nil, fmt.Errorf("failed to unmarshal response: %w", err)
	}

	return &result, nil
}

// List retrieves a list of payment intents.
func (s *PaymentIntentsService) List(params *ListPaymentParams) (*PaymentIntentList, error) {
	body, err := s.client.doRequest("GET", "/v1/payment-intents", params)
	if err != nil {
		return nil, err
	}

	var result PaymentIntentList
	if err := json.Unmarshal(body, &result); err != nil {
		return nil, fmt.Errorf("failed to unmarshal response: %w", err)
	}

	return &result, nil
}

// Capture captures a payment intent.
func (s *PaymentIntentsService) Capture(id string, amount *int64) (*PaymentIntent, error) {
	params := map[string]interface{}{}
	if amount != nil {
		params["amount"] = *amount
	}

	body, err := s.client.doRequest("POST", fmt.Sprintf("/v1/payment-intents/%s/capture", id), params)
	if err != nil {
		return nil, err
	}

	var result PaymentIntent
	if err := json.Unmarshal(body, &result); err != nil {
		return nil, fmt.Errorf("failed to unmarshal response: %w", err)
	}

	return &result, nil
}

// Void voids (cancels) a payment intent.
func (s *PaymentIntentsService) Void(id string) (*PaymentIntent, error) {
	body, err := s.client.doRequest("POST", fmt.Sprintf("/v1/payment-intents/%s/void", id), nil)
	if err != nil {
		return nil, err
	}

	var result PaymentIntent
	if err := json.Unmarshal(body, &result); err != nil {
		return nil, fmt.Errorf("failed to unmarshal response: %w", err)
	}

	return &result, nil
}

// Refund refunds a payment intent.
func (s *PaymentIntentsService) Refund(id string, amount *int64) (*PaymentIntent, error) {
	params := map[string]interface{}{}
	if amount != nil {
		params["amount"] = *amount
	}

	body, err := s.client.doRequest("POST", fmt.Sprintf("/v1/payment-intents/%s/refund", id), params)
	if err != nil {
		return nil, err
	}

	var result PaymentIntent
	if err := json.Unmarshal(body, &result); err != nil {
		return nil, fmt.Errorf("failed to unmarshal response: %w", err)
	}

	return &result, nil
}
