//! Notification templates — value object with variable substitution.

use serde::{Deserialize, Serialize};

use super::channel::NotificationChannel;

/// A notification template with subject and body. Supports variable substitution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationTemplate {
    pub template_id: String,
    pub channel: NotificationChannel,
    pub subject_template: Option<String>,
    pub body_template: String,
    pub description: String,
}

impl NotificationTemplate {
    /// Render a template by substituting variables in `{{var_name}}` format.
    pub fn render(&self, variables: &std::collections::HashMap<String, String>) -> RenderedMessage {
        let mut subject = self.subject_template.clone();
        let mut body = self.body_template.clone();

        for (key, value) in variables {
            let placeholder = format!("{{{{{}}}}}", key);
            if let Some(ref mut s) = subject {
                *s = s.replace(&placeholder, value);
            }
            body = body.replace(&placeholder, value);
        }

        RenderedMessage { subject, body }
    }
}

/// A fully rendered notification message.
#[derive(Debug, Clone)]
pub struct RenderedMessage {
    pub subject: Option<String>,
    pub body: String,
}

/// Default notification templates for common events.
pub fn default_templates() -> Vec<NotificationTemplate> {
    vec![
        NotificationTemplate {
            template_id: "direct_email".into(),
            channel: NotificationChannel::Email,
            subject_template: None,
            body_template: "{{body}}".into(),
            description: "Direct email send (no template)".into(),
        },
        NotificationTemplate {
            template_id: "direct_sms".into(),
            channel: NotificationChannel::Sms,
            subject_template: None,
            body_template: "{{body}}".into(),
            description: "Direct SMS send (no template)".into(),
        },
        NotificationTemplate {
            template_id: "payment_failed".into(),
            channel: NotificationChannel::Email,
            subject_template: Some("[Payment Orchestra] Payment Failed - {{payment_intent_id}}".into()),
            body_template: "Payment {{payment_intent_id}} failed on all acquirers.\nAmount: {{amount}}\nOperator: {{operator_name}}\n\nPlease review the transaction in the dashboard.".into(),
            description: "Payment failed on all acquirers".into(),
        },
        NotificationTemplate {
            template_id: "settlement_unmatched".into(),
            channel: NotificationChannel::Email,
            subject_template: Some("[Payment Orchestra] Unmatched Settlement - {{batch_id}}".into()),
            body_template: "Settlement record {{transaction_id}} in batch {{batch_id}} is unmatched.\nAmount: {{amount}}\n\nPlease review in the reconciliation dashboard.".into(),
            description: "Unmatched settlement record detected".into(),
        },
        NotificationTemplate {
            template_id: "chargeback_received".into(),
            channel: NotificationChannel::Email,
            subject_template: Some("[Payment Orchestra] Chargeback Received - {{payment_intent_id}}".into()),
            body_template: "A chargeback has been received for payment {{payment_intent_id}}.\nReason: {{reason_code}}\nAmount: {{amount}}\nRepresentment deadline: {{deadline}}\n\nPlease review in the dispute dashboard.".into(),
            description: "New chargeback received".into(),
        },
        NotificationTemplate {
            template_id: "subscription_failed".into(),
            channel: NotificationChannel::Email,
            subject_template: Some("[Payment Orchestra] Subscription Payment Failed".into()),
            body_template: "Your subscription payment of {{amount}} has failed.\nSubscription: {{subscription_id}}\nPlease update your payment method to avoid interruption.".into(),
            description: "Subscription renewal payment failed".into(),
        },
        NotificationTemplate {
            template_id: "api_key_expiring".into(),
            channel: NotificationChannel::Email,
            subject_template: Some("[Payment Orchestra] API Key Expiring - {{key_name}}".into()),
            body_template: "Your API key '{{key_name}}' is expiring in {{days_remaining}} days.\nPlease rotate your key before it expires.".into(),
            description: "API key is expiring soon".into(),
        },
    ]
}
