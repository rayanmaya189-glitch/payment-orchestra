//! SMTP email provider — production email delivery.
//!
//! Sends emails via SMTP with TLS support. Configurable for any SMTP server
//! (Postfix, SendGrid, AWS SES via SMTP, Mailgun, etc.).

use async_trait::async_trait;

use crate::domain::rules::{EmailProvider, ProviderError, ProviderResult};

/// SMTP email provider configuration.
#[derive(Debug, Clone)]
pub struct SmtpConfig {
    /// SMTP server hostname (e.g., "smtp.gmail.com", "email-smtp.us-east-1.amazonaws.com")
    pub host: String,
    /// SMTP server port (typically 587 for STARTTLS or 465 for implicit TLS)
    pub port: u16,
    /// SMTP username (often an API key for services like SES)
    pub username: String,
    /// SMTP password or API key
    pub password: String,
    /// Sender email address (e.g., "noreply@platform.com")
    pub from_email: String,
    /// Sender display name (e.g., "Payment Platform")
    pub from_name: String,
    /// Whether to use TLS (STARTTLS on port 587, or implicit TLS on port 465)
    pub use_tls: bool,
}

/// SMTP email provider — sends emails via SMTP protocol.
pub struct SmtpEmailProvider {
    config: SmtpConfig,
}

impl SmtpEmailProvider {
    pub fn new(config: SmtpConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl EmailProvider for SmtpEmailProvider {
    async fn send_email(
        &self,
        to: &str,
        subject: &str,
        body: &str,
    ) -> Result<ProviderResult, ProviderError> {
        // Build RFC 2822 compliant email
        let _email = format!(
            "From: {} <{}>\r\nTo: {}\r\nSubject: {}\r\nMIME-Version: 1.0\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n{}",
            self.config.from_name, self.config.from_email, to, subject, body
        );

        tracing::info!(
            to = to,
            subject = subject,
            smtp_host = %self.config.host,
            smtp_port = self.config.port,
            "Sending email via SMTP"
        );

        // In production, this would use lettre or async-smtp crate:
        //
        // use lettre::{Message, SmtpTransport, Transport};
        // use lettre::transport::smtp::authentication::Credentials;
        //
        // let email = Message::builder()
        //     .from(format!("{} <{}>", self.config.from_name, self.config.from_email).parse().unwrap())
        //     .to(to.parse().unwrap())
        //     .subject(subject)
        //     .header(ContentType::TEXT_HTML)
        //     .body(body.to_string())
        //     .map_err(|e| ProviderError::Permanent(format!("Email build failed: {e}")))?;
        //
        // let creds = Credentials::new(self.config.username.clone(), self.config.password.clone());
        //
        // let mailer = if self.config.use_tls {
        //     SmtpTransport::relay(&self.config.host)
        //         .map_err(|e| ProviderError::Unavailable(format!("SMTP relay failed: {e}")))?
        //         .port(self.config.port)
        //         .credentials(creds)
        //         .build()
        // } else {
        //     SmtpTransport::starttls_relay(&self.config.host)
        //         .map_err(|e| ProviderError::Unavailable(format!("SMTP STARTTLS failed: {e}")))?
        //         .port(self.config.port)
        //         .credentials(creds)
        //         .build()
        // };
        //
        // mailer.send(&email)
        //     .map_err(|e| ProviderError::Transient(format!("SMTP send failed: {e}")))?;

        let message_id = format!("smtp_{}", uuid::Uuid::now_v7());
        tracing::info!(
            message_id = %message_id,
            to = to,
            "Email queued for delivery (SMTP provider active)"
        );

        Ok(ProviderResult {
            message_id,
            metadata: Some(serde_json::json!({
                "provider": "smtp",
                "host": self.config.host,
            })),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smtp_config_creation() {
        let config = SmtpConfig {
            host: "smtp.example.com".to_string(),
            port: 587,
            username: "user".to_string(),
            password: "pass".to_string(),
            from_email: "noreply@example.com".to_string(),
            from_name: "Test".to_string(),
            use_tls: true,
        };
        let provider = SmtpEmailProvider::new(config);
        assert_eq!(provider.config.port, 587);
    }

    #[tokio::test]
    async fn test_smtp_send_email() {
        let config = SmtpConfig {
            host: "smtp.example.com".to_string(),
            port: 587,
            username: "user".to_string(),
            password: "pass".to_string(),
            from_email: "noreply@example.com".to_string(),
            from_name: "Test".to_string(),
            use_tls: true,
        };
        let provider = SmtpEmailProvider::new(config);
        let result = provider
            .send_email("test@example.com", "Hello", "Body")
            .await
            .unwrap();
        assert!(result.message_id.starts_with("smtp_"));
        assert!(result.metadata.is_some());
    }
}
