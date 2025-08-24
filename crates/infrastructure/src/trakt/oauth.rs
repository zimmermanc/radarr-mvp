use chrono::{DateTime, Utc};
use radarr_core::{
    streaming::{OAuthToken, TraktDeviceCode, TraktTokenResponse},
    RadarrError,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};

/// Trakt OAuth configuration
pub struct TraktOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

impl TraktOAuthConfig {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            client_id,
            client_secret,
            redirect_uri: "urn:ietf:wg:oauth:2.0:oob".to_string(), // For device flow
        }
    }
}

/// Trakt OAuth device flow implementation
pub struct TraktOAuth {
    client: Client,
    config: TraktOAuthConfig,
    base_url: String,
}

impl TraktOAuth {
    pub fn new(config: TraktOAuthConfig) -> Self {
        Self {
            client: Client::new(),
            config,
            base_url: "https://api.trakt.tv".to_string(),
        }
    }

    /// Initiate device flow authentication
    pub async fn initiate_device_flow(&self) -> Result<TraktDeviceCode, RadarrError> {
        let url = format!("{}/oauth/device/code", self.base_url);

        info!("Initiating Trakt device flow authentication");

        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "client_id": self.config.client_id
            }))
            .send()
            .await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "trakt".to_string(),
                error: e.to_string(),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("Trakt device flow initiation failed: {} - {}", status, text);
            return Err(RadarrError::ExternalServiceError {
                service: "trakt".to_string(),
                error: format!("HTTP {}: {}", status, text),
            });
        }

        let device_response: DeviceCodeResponse =
            response
                .json()
                .await
                .map_err(|e| RadarrError::ExternalServiceError {
                    service: "trakt".to_string(),
                    error: e.to_string(),
                })?;

        info!(
            "Device flow initiated. User code: {}, Verification URL: {}",
            device_response.user_code, device_response.verification_url
        );

        Ok(TraktDeviceCode {
            device_code: device_response.device_code,
            user_code: device_response.user_code,
            verification_url: device_response.verification_url,
            expires_in: device_response.expires_in,
            interval: device_response.interval,
        })
    }

    /// Poll for token after user has authorized the device
    pub async fn poll_for_token(
        &self,
        device_code: &str,
    ) -> Result<TraktTokenResponse, RadarrError> {
        let url = format!("{}/oauth/device/token", self.base_url);

        debug!("Polling for Trakt token");

        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "code": device_code,
                "client_id": self.config.client_id,
                "client_secret": self.config.client_secret
            }))
            .send()
            .await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "trakt".to_string(),
                error: e.to_string(),
            })?;

        match response.status().as_u16() {
            200 => {
                let token_response: TokenResponse =
                    response
                        .json()
                        .await
                        .map_err(|e| RadarrError::ExternalServiceError {
                            service: "trakt".to_string(),
                            error: e.to_string(),
                        })?;

                info!("Successfully obtained Trakt access token");

                Ok(TraktTokenResponse {
                    access_token: token_response.access_token,
                    token_type: token_response.token_type,
                    expires_in: token_response.expires_in,
                    refresh_token: token_response.refresh_token,
                    scope: token_response.scope,
                    created_at: token_response.created_at,
                })
            }
            400 => {
                // Pending - user hasn't authorized yet
                debug!("Authorization pending");
                Err(RadarrError::ExternalServiceError {
                    service: "trakt".to_string(),
                    error: "Authorization pending".to_string(),
                })
            }
            404 => {
                // Invalid or expired code
                error!("Invalid or expired device code");
                Err(RadarrError::ExternalServiceError {
                    service: "trakt".to_string(),
                    error: "Invalid or expired device code".to_string(),
                })
            }
            409 => {
                // Already used code
                error!("Device code already used");
                Err(RadarrError::ExternalServiceError {
                    service: "trakt".to_string(),
                    error: "Device code already used".to_string(),
                })
            }
            410 => {
                // Expired
                error!("Device code expired");
                Err(RadarrError::ExternalServiceError {
                    service: "trakt".to_string(),
                    error: "Device code expired".to_string(),
                })
            }
            418 => {
                // User denied
                error!("User denied authorization");
                Err(RadarrError::ExternalServiceError {
                    service: "trakt".to_string(),
                    error: "User denied authorization".to_string(),
                })
            }
            429 => {
                // Rate limit - slow down
                warn!("Rate limited, slow down polling");
                Err(RadarrError::RateLimited {
                    service: "trakt".to_string(),
                    retry_after: None,
                })
            }
            _ => {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                error!("Unexpected response: {} - {}", status, text);
                Err(RadarrError::ExternalServiceError {
                    service: "trakt".to_string(),
                    error: format!("HTTP {}: {}", status, text),
                })
            }
        }
    }

    /// Poll for token with automatic retry and interval management
    pub async fn poll_for_token_with_retry(
        &self,
        device_code: &str,
        expires_in: i32,
        interval: i32,
    ) -> Result<TraktTokenResponse, RadarrError> {
        let expiry_time = Utc::now() + chrono::Duration::seconds(expires_in as i64);
        let poll_interval = Duration::from_secs(interval as u64);

        info!(
            "Starting token polling with {}s interval, expires in {}s",
            interval, expires_in
        );

        while Utc::now() < expiry_time {
            match self.poll_for_token(device_code).await {
                Ok(token) => return Ok(token),
                Err(RadarrError::ExternalServiceError { error, .. })
                    if error.contains("Authorization pending") =>
                {
                    debug!("Still waiting for user authorization...");
                    sleep(poll_interval).await;
                    continue;
                }
                Err(RadarrError::RateLimited { .. }) => {
                    // Slow down if rate limited
                    warn!("Rate limited, doubling poll interval");
                    sleep(poll_interval * 2).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        Err(RadarrError::ExternalServiceError {
            service: "trakt".to_string(),
            error: "Device code expired before user authorization".to_string(),
        })
    }

    /// Refresh an expired or expiring token
    pub async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<TraktTokenResponse, RadarrError> {
        let url = format!("{}/oauth/token", self.base_url);

        info!("Refreshing Trakt access token");

        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "refresh_token": refresh_token,
                "client_id": self.config.client_id,
                "client_secret": self.config.client_secret,
                "redirect_uri": self.config.redirect_uri,
                "grant_type": "refresh_token"
            }))
            .send()
            .await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "trakt".to_string(),
                error: e.to_string(),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("Token refresh failed: {} - {}", status, text);
            return Err(RadarrError::ExternalServiceError {
                service: "trakt".to_string(),
                error: format!("HTTP {}: {}", status, text),
            });
        }

        let token_response: TokenResponse =
            response
                .json()
                .await
                .map_err(|e| RadarrError::ExternalServiceError {
                    service: "trakt".to_string(),
                    error: e.to_string(),
                })?;

        info!("Successfully refreshed Trakt access token");

        Ok(TraktTokenResponse {
            access_token: token_response.access_token,
            token_type: token_response.token_type,
            expires_in: token_response.expires_in,
            refresh_token: token_response.refresh_token,
            scope: token_response.scope,
            created_at: token_response.created_at,
        })
    }

    /// Convert token response to OAuthToken for storage
    pub fn token_to_oauth(&self, token: TraktTokenResponse) -> OAuthToken {
        let expires_at = DateTime::<Utc>::from_timestamp(token.created_at, 0)
            .unwrap_or_else(Utc::now)
            + chrono::Duration::seconds(token.expires_in as i64);

        OAuthToken {
            id: None,
            service: "trakt".to_string(),
            access_token: token.access_token,
            refresh_token: Some(token.refresh_token),
            token_type: token.token_type,
            expires_at,
            scope: Some(token.scope),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

// Response models for Trakt OAuth
#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_url: String,
    expires_in: i32,
    interval: i32,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: i32,
    refresh_token: String,
    scope: String,
    created_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_config_creation() {
        let config = TraktOAuthConfig::new("client_id".to_string(), "client_secret".to_string());

        assert_eq!(config.client_id, "client_id");
        assert_eq!(config.client_secret, "client_secret");
        assert_eq!(config.redirect_uri, "urn:ietf:wg:oauth:2.0:oob");
    }

    #[test]
    fn test_token_to_oauth_conversion() {
        let config = TraktOAuthConfig::new("client_id".to_string(), "client_secret".to_string());
        let oauth = TraktOAuth::new(config);

        let token_response = TraktTokenResponse {
            access_token: "access_token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 7776000, // 90 days
            refresh_token: "refresh_token".to_string(),
            scope: "public".to_string(),
            created_at: Utc::now().timestamp(),
        };

        let oauth_token = oauth.token_to_oauth(token_response);

        assert_eq!(oauth_token.service, "trakt");
        assert_eq!(oauth_token.access_token, "access_token");
        assert_eq!(oauth_token.refresh_token, Some("refresh_token".to_string()));
        assert!(oauth_token.expires_at > Utc::now());
    }
}
