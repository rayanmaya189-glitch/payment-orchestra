//! Stripe API client for billing integration.
//!
//! This module provides a client for interacting with the Stripe API
//! to manage subscriptions, invoices, and payment methods.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Stripe API client configuration.
#[derive(Debug, Clone)]
pub struct StripeConfig {
    /// Stripe secret API key
    pub secret_key: String,
    /// Stripe API version
    pub api_version: String,
    /// Base URL (for testing with Stripe mock)
    pub base_url: String,
}

impl Default for StripeConfig {
    fn default() -> Self {
        Self {
            secret_key: std::env::var("STRIPE_SECRET_KEY").unwrap_or_default(),
            api_version: "2024-12-18".to_string(),
            base_url: "https://api.stripe.com".to_string(),
        }
    }
}

/// Stripe API client.
pub struct StripeClient {
    client: Client,
    config: StripeConfig,
}

/// Stripe Customer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeCustomer {
    pub id: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub metadata: std::collections::HashMap<String, String>,
}

/// Stripe Subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeSubscription {
    pub id: String,
    pub customer: String,
    pub status: String,
    pub current_period_start: i64,
    pub current_period_end: i64,
    pub items: StripeSubscriptionItems,
    pub metadata: std::collections::HashMap<String, String>,
}

/// Stripe Subscription Items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeSubscriptionItems {
    pub data: Vec<StripeSubscriptionItem>,
}

/// Stripe Subscription Item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeSubscriptionItem {
    pub id: String,
    pub price: StripePrice,
    pub quantity: Option<u32>,
}

/// Stripe Price.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripePrice {
    pub id: String,
    pub unit_amount: Option<u64>,
    pub currency: String,
    pub recurring: Option<StripeRecurring>,
}

/// Stripe Recurring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeRecurring {
    pub interval: String,
    pub interval_count: Option<u32>,
}

/// Stripe Invoice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeInvoice {
    pub id: String,
    pub customer: String,
    pub subscription: Option<String>,
    pub status: String,
    pub amount_due: u64,
    pub amount_paid: u64,
    pub currency: String,
    pub payment_intent: Option<String>,
    pub hosted_invoice_url: Option<String>,
}

/// Stripe Payment Intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripePaymentIntent {
    pub id: String,
    pub amount: u64,
    pub currency: String,
    pub status: String,
    pub client_secret: Option<String>,
}

/// Stripe Error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeError {
    pub error: StripeErrorDetail,
}

/// Stripe Error Detail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeErrorDetail {
    pub code: Option<String>,
    pub message: Option<String>,
    pub param: Option<String>,
    pub r#type: String,
}

impl std::fmt::Display for StripeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Stripe Error: {} - {}",
            self.error.r#type,
            self.error.message.as_deref().unwrap_or("Unknown error")
        )
    }
}

impl std::error::Error for StripeError {}

impl StripeClient {
    /// Create a new Stripe client.
    pub fn new(config: StripeConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, config }
    }

    /// Get the base URL for API requests.
    fn base_url(&self) -> &str {
        &self.config.base_url
    }

    /// Get the authorization header.
    fn auth_header(&self) -> String {
        format!("Bearer {}", self.config.secret_key)
    }

    /// Create a customer in Stripe.
    pub async fn create_customer(
        &self,
        email: &str,
        name: &str,
        metadata: std::collections::HashMap<String, String>,
    ) -> Result<StripeCustomer, StripeError> {
        let url = format!("{}/v1/customers", self.base_url());
        
        let mut form = reqwest::multipart::Form::new()
            .text("email", email.to_string())
            .text("name", name.to_string());

        for (key, value) in &metadata {
            form = form.text(format!("metadata[{}]", key), value.clone());
        }

        let response = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .multipart(form)
            .send()
            .await
            .map_err(|e| StripeError {
                error: StripeErrorDetail {
                    code: None,
                    message: Some(format!("Request failed: {}", e)),
                    param: None,
                    r#type: "network_error".to_string(),
                },
            })?;

        if response.status().is_success() {
            response.json().await.map_err(|e: reqwest::Error| StripeError {
                error: StripeErrorDetail {
                    code: None,
                    message: Some(format!("Failed to parse response: {}", e)),
                    param: None,
                    r#type: "parse_error".to_string(),
                },
            })
        } else {
            response.json().await.map_err(|e: reqwest::Error| StripeError {
                error: StripeErrorDetail {
                    code: None,
                    message: Some(format!("Failed to parse error response: {}", e)),
                    param: None,
                    r#type: "api_error".to_string(),
                },
            })
        }
    }

    /// Create a subscription in Stripe.
    pub async fn create_subscription(
        &self,
        customer_id: &str,
        price_id: &str,
        trial_days: Option<u32>,
        metadata: std::collections::HashMap<String, String>,
    ) -> Result<StripeSubscription, StripeError> {
        let url = format!("{}/v1/subscriptions", self.base_url());
        
        let mut form = reqwest::multipart::Form::new()
            .text("customer", customer_id.to_string())
            .text("items[0][price]", price_id.to_string());

        if let Some(days) = trial_days {
            form = form.text("trial_period_days", days.to_string());
        }

        for (key, value) in &metadata {
            form = form.text(format!("metadata[{}]", key), value.clone());
        }

        let response = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .multipart(form)
            .send()
            .await
            .map_err(|e| StripeError {
                error: StripeErrorDetail {
                    code: None,
                    message: Some(format!("Request failed: {}", e)),
                    param: None,
                    r#type: "network_error".to_string(),
                },
            })?;

        if response.status().is_success() {
            response.json().await.map_err(|e: reqwest::Error| StripeError {
                error: StripeErrorDetail {
                    code: None,
                    message: Some(format!("Failed to parse response: {}", e)),
                    param: None,
                    r#type: "parse_error".to_string(),
                },
            })
        } else {
            response.json().await.map_err(|e: reqwest::Error| StripeError {
                error: StripeErrorDetail {
                    code: None,
                    message: Some(format!("Failed to parse error response: {}", e)),
                    param: None,
                    r#type: "api_error".to_string(),
                },
            })
        }
    }

    /// Cancel a subscription in Stripe.
    pub async fn cancel_subscription(
        &self,
        subscription_id: &str,
        at_period_end: bool,
    ) -> Result<StripeSubscription, StripeError> {
        let url = format!("{}/v1/subscriptions/{}", self.base_url(), subscription_id);
        
        let mut form = reqwest::multipart::Form::new();
        
        if at_period_end {
            form = form.text("cancel_at_period_end", "true".to_string());
        } else {
            form = form.text("cancel_at_period_end", "false".to_string());
        }

        let response = self
            .client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .multipart(form)
            .send()
            .await
            .map_err(|e| StripeError {
                error: StripeErrorDetail {
                    code: None,
                    message: Some(format!("Request failed: {}", e)),
                    param: None,
                    r#type: "network_error".to_string(),
                },
            })?;

        if response.status().is_success() {
            response.json().await.map_err(|e: reqwest::Error| StripeError {
                error: StripeErrorDetail {
                    code: None,
                    message: Some(format!("Failed to parse response: {}", e)),
                    param: None,
                    r#type: "parse_error".to_string(),
                },
            })
        } else {
            response.json().await.map_err(|e: reqwest::Error| StripeError {
                error: StripeErrorDetail {
                    code: None,
                    message: Some(format!("Failed to parse error response: {}", e)),
                    param: None,
                    r#type: "api_error".to_string(),
                },
            })
        }
    }

    /// Update a subscription in Stripe.
    pub async fn update_subscription(
        &self,
        subscription_id: &str,
        price_id: Option<&str>,
        metadata: Option<std::collections::HashMap<String, String>>,
    ) -> Result<StripeSubscription, StripeError> {
        let url = format!("{}/v1/subscriptions/{}", self.base_url(), subscription_id);
        
        let mut form = reqwest::multipart::Form::new();

        if let Some(price) = price_id {
            // First, get current subscription to find item ID
            let sub = self.get_subscription(subscription_id).await?;
            if let Some(item) = sub.items.data.first() {
                form = form.text("items[0][id]", item.id.clone());
                form = form.text("items[0][price]", price.to_string());
            }
        }

        if let Some(meta) = metadata {
            for (key, value) in &meta {
                form = form.text(format!("metadata[{}]", key), value.clone());
            }
        }

        let response = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .multipart(form)
            .send()
            .await
            .map_err(|e| StripeError {
                error: StripeErrorDetail {
                    code: None,
                    message: Some(format!("Request failed: {}", e)),
                    param: None,
                    r#type: "network_error".to_string(),
                },
            })?;

        if response.status().is_success() {
            response.json().await.map_err(|e: reqwest::Error| StripeError {
                error: StripeErrorDetail {
                    code: None,
                    message: Some(format!("Failed to parse response: {}", e)),
                    param: None,
                    r#type: "parse_error".to_string(),
                },
            })
        } else {
            response.json().await.map_err(|e: reqwest::Error| StripeError {
                error: StripeErrorDetail {
                    code: None,
                    message: Some(format!("Failed to parse error response: {}", e)),
                    param: None,
                    r#type: "api_error".to_string(),
                },
            })
        }
    }

    /// Get a subscription from Stripe.
    pub async fn get_subscription(
        &self,
        subscription_id: &str,
    ) -> Result<StripeSubscription, StripeError> {
        let url = format!("{}/v1/subscriptions/{}", self.base_url(), subscription_id);

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| StripeError {
                error: StripeErrorDetail {
                    code: None,
                    message: Some(format!("Request failed: {}", e)),
                    param: None,
                    r#type: "network_error".to_string(),
                },
            })?;

        if response.status().is_success() {
            response.json().await.map_err(|e: reqwest::Error| StripeError {
                error: StripeErrorDetail {
                    code: None,
                    message: Some(format!("Failed to parse response: {}", e)),
                    param: None,
                    r#type: "parse_error".to_string(),
                },
            })
        } else {
            response.json().await.map_err(|e: reqwest::Error| StripeError {
                error: StripeErrorDetail {
                    code: None,
                    message: Some(format!("Failed to parse error response: {}", e)),
                    param: None,
                    r#type: "api_error".to_string(),
                },
            })
        }
    }

    /// Get an invoice from Stripe.
    pub async fn get_invoice(
        &self,
        invoice_id: &str,
    ) -> Result<StripeInvoice, StripeError> {
        let url = format!("{}/v1/invoices/{}", self.base_url(), invoice_id);

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| StripeError {
                error: StripeErrorDetail {
                    code: None,
                    message: Some(format!("Request failed: {}", e)),
                    param: None,
                    r#type: "network_error".to_string(),
                },
            })?;

        if response.status().is_success() {
            response.json().await.map_err(|e: reqwest::Error| StripeError {
                error: StripeErrorDetail {
                    code: None,
                    message: Some(format!("Failed to parse response: {}", e)),
                    param: None,
                    r#type: "parse_error".to_string(),
                },
            })
        } else {
            response.json().await.map_err(|e: reqwest::Error| StripeError {
                error: StripeErrorDetail {
                    code: None,
                    message: Some(format!("Failed to parse error response: {}", e)),
                    param: None,
                    r#type: "api_error".to_string(),
                },
            })
        }
    }

    /// Create a payment intent for one-time payment.
    pub async fn create_payment_intent(
        &self,
        amount: u64,
        currency: &str,
        customer_id: Option<&str>,
        metadata: std::collections::HashMap<String, String>,
    ) -> Result<StripePaymentIntent, StripeError> {
        let url = format!("{}/v1/payment_intents", self.base_url());
        
        let mut form = reqwest::multipart::Form::new()
            .text("amount", amount.to_string())
            .text("currency", currency.to_string())
            .text("automatic_payment_methods[enabled]", "true".to_string());

        if let Some(customer) = customer_id {
            form = form.text("customer", customer.to_string());
        }

        for (key, value) in &metadata {
            form = form.text(format!("metadata[{}]", key), value.clone());
        }

        let response = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .multipart(form)
            .send()
            .await
            .map_err(|e| StripeError {
                error: StripeErrorDetail {
                    code: None,
                    message: Some(format!("Request failed: {}", e)),
                    param: None,
                    r#type: "network_error".to_string(),
                },
            })?;

        if response.status().is_success() {
            response.json().await.map_err(|e: reqwest::Error| StripeError {
                error: StripeErrorDetail {
                    code: None,
                    message: Some(format!("Failed to parse response: {}", e)),
                    param: None,
                    r#type: "parse_error".to_string(),
                },
            })
        } else {
            response.json().await.map_err(|e: reqwest::Error| StripeError {
                error: StripeErrorDetail {
                    code: None,
                    message: Some(format!("Failed to parse error response: {}", e)),
                    param: None,
                    r#type: "api_error".to_string(),
                },
            })
        }
    }
}
