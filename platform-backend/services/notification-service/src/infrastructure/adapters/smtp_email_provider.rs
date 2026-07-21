//! SMTP email provider — production email delivery via lettre.
//!
//! Sends emails through an SMTP server using the `lettre` crate with
//! TLS/STARTTLS support. Compatible with Postfix, SendGrid (via SMTP),
//! AWS SES, Mailgun, and any standard SMTP server.

use async_trait::async_trait;
use lettre::message::{header::ContentType, Mailbox};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

use crate::domain::rules::{EmailProvider, ProviderError, ProviderResult};

/// SMTP email provider configuration.
#[derive(Debug, Clone)]
pub struct SmtpConfig {
    /// SMTP server hostname (e.g. `smtp.gmail.com`, `email-smtp.us-east-1.amazonaws.com`)
    pub host: String,
    /// SMTP server port (587 for STARTTLS, 465 for implicit TLS)
    pub port: u16,
    /// SMTP username (often an API key for services like SES)
    pub username: String,
    /// SMTP password or API key
    pub password: String,
    /// Sender email address (e.g. `noreply@platform.com`)
    pub from_email: String,
    /// Sender display name (e.g. `Payment Platform`)
    pub from_name: String,
    /// Whether to use implicit TLS (port 465) vs STARTTLS (port 587)
    pub use_tls: bool,
}

/// SMTP email provider — sends emails via the lettre SMTP transport.
pub struct SmtpEmailProvider {
    config: SmtpConfig,
}

impl SmtpEmailProvider {
    pub fn new(config: SmtpConfig) -> Self {
        Self { config }
    }

    /// Build the lettre `Message` from the provided fields.
    fn build_message(
        &self,
        to: &str,
        subject: &str,
        body: &str,
    ) -> Result<Message, ProviderError> {
        let from: Mailbox = format!("{} <{}>", self.config.from_name, self.config.from_email)
            .parse()
            .map_err(|e| ProviderError::Permanent(format!("Invalid from address: {e}")))?;

        let to_addr: Mailbox = to
            .parse()
            .map_err(|e| ProviderError::Permanent(format!("Invalid to address: {e}")))?;

        Message::builder()
            .from(from)
            .to(to_addr)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(body.to_string())
            .map_err(|e| ProviderError::Permanent(format!("Email build failed: {e}")))
    }

    /// Build the SMTP transport from the configuration.
    fn build_transport(&self) -> Result<SmtpTransport, ProviderError> {
        let creds = Credentials::new(self.config.username.clone(), self.config.password.clone());

        if self.config.use_tls {
            let builder = SmtpTransport::relay(&self.config.host)
                .map_err(|e| ProviderError::Unavailable(format!("SMTP relay failed: {e}")))?;
            Ok(builder.port(self.config.port).credentials(creds).build())
        } else {
            let builder = SmtpTransport::starttls_relay(&self.config.host)
                .map_err(|e| ProviderError::Unavailable(format!("SMTP STARTTLS relay failed: {e}")))?;
            Ok(builder.port(self.config.port).credentials(creds).build())
        }
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
        tracing::info!(
            to = to,
            subject = subject,
            smtp_host = %self.config.host,
            smtp_port = self.config.port,
            "Sending email via SMTP"
        );

        let email = self.build_message(to, subject, body)?;
        let transport = self.build_transport()?;

        transport
            .send(&email)
            .map_err(|e| {
                tracing::error!(
                    to = to,
                    error = %e,
                    "SMTP send failed"
                );
                // Distinguish transient from permanent errors
                let err_str = e.to_string();
                if err_str.contains("connection")
                    || err_str.contains("timeout")
                    || err_str.contains("refused")
                {
                    ProviderError::Transient(format!("SMTP send failed: {e}"))
                } else {
                    ProviderError::Permanent(format!("SMTP send failed: {e}"))
                }
            })?;

        let message_id = format!("smtp_{}", uuid::Uuid::now_v7());
        tracing::info!(
            message_id = %message_id,
            to = to,
            "Email sent successfully via SMTP"
        );

        Ok(ProviderResult {
            message_id,
            metadata: Some(serde_json::json!({
                "provider": "smtp",
                "host": self.config.host,
                "port": self.config.port,
                "from": self.config.from_email,
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

    #[test]
    fn test_build_message() {
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
        let result = provider.build_message("test@example.com", "Hello", "<p>Body</p>");
        assert!(result.is_ok());
    }
}
