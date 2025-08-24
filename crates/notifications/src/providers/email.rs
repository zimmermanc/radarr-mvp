use async_trait::async_trait;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

use crate::models::{Notification, NotificationError, NotificationProvider, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub enabled: bool,
    pub smtp_server: String,
    pub smtp_port: u16,
    pub username: String,
    pub password: String,
    pub from_address: String,
    pub from_name: Option<String>,
    pub to_addresses: Vec<String>,
    pub use_tls: bool,
}

pub struct EmailProvider {
    config: EmailConfig,
    smtp: Option<AsyncSmtpTransport<Tokio1Executor>>,
}

impl EmailProvider {
    pub fn new(config: EmailConfig) -> Self {
        let smtp = if config.enabled {
            Self::create_smtp_client(&config).ok()
        } else {
            None
        };

        Self { config, smtp }
    }

    fn create_smtp_client(config: &EmailConfig) -> Result<AsyncSmtpTransport<Tokio1Executor>> {
        let creds = Credentials::new(config.username.clone(), config.password.clone());

        let transport = if config.use_tls {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_server)
                .map_err(|e| NotificationError::EmailError(e.to_string()))?
                .credentials(creds)
                .build()
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.smtp_server)
                .port(config.smtp_port)
                .credentials(creds)
                .build()
        };

        Ok(transport)
    }
}

#[async_trait]
impl NotificationProvider for EmailProvider {
    fn name(&self) -> &str {
        "Email"
    }

    fn is_enabled(&self) -> bool {
        self.config.enabled && self.smtp.is_some()
    }

    async fn test(&self) -> Result<()> {
        if !self.is_enabled() {
            return Err(NotificationError::ProviderUnavailable(
                "Email provider is disabled or not configured".to_string(),
            ));
        }

        // Send a test email
        let test_notification = Notification::new(
            crate::NotificationEventType::HealthCheckFailed,
            "Test Email".to_string(),
            "This is a test email from Radarr MVP".to_string(),
            crate::NotificationData::Health(crate::HealthNotificationData {
                check_name: "Email Integration".to_string(),
                status: "success".to_string(),
                message: "Email provider is configured correctly".to_string(),
                details: None,
            }),
        );

        self.send(&test_notification).await
    }

    async fn send(&self, notification: &Notification) -> Result<()> {
        if !self.is_enabled() {
            debug!("Email provider is disabled, skipping notification");
            return Ok(());
        }

        let smtp = self.smtp.as_ref().unwrap();

        for to_address in &self.config.to_addresses {
            let subject = format!("[Radarr] {}", notification.title);
            let body = self.create_email_body(notification);

            let from_name = self.config.from_name.as_deref().unwrap_or("Radarr MVP");
            let from = format!("{} <{}>", from_name, self.config.from_address);

            let email = Message::builder()
                .from(from.parse().map_err(|e| {
                    NotificationError::EmailError(format!("Invalid from address: {}", e))
                })?)
                .to(to_address.parse().map_err(|e| {
                    NotificationError::EmailError(format!("Invalid to address: {}", e))
                })?)
                .subject(subject)
                .body(body)
                .map_err(|e| NotificationError::EmailError(e.to_string()))?;

            match smtp.send(email).await {
                Ok(_) => {
                    info!("Successfully sent email notification to {}", to_address);
                }
                Err(e) => {
                    error!("Failed to send email to {}: {}", to_address, e);
                    return Err(NotificationError::EmailError(format!(
                        "Failed to send email: {}",
                        e
                    )));
                }
            }
        }

        Ok(())
    }
}

impl EmailProvider {
    fn create_email_body(&self, notification: &Notification) -> String {
        format!(
            "Subject: {}\n\nMessage: {}\n\nEvent Type: {:?}\n\nTimestamp: {}\n\n---\nThis email was sent by Radarr MVP",
            notification.title,
            notification.message,
            notification.event_type,
            notification.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        )
    }
}
