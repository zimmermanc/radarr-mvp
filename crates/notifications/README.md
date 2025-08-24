# Radarr Notifications

Comprehensive notification system for the Radarr movie automation system. This crate provides multiple notification providers including email, webhooks, and messaging services with templating support and delivery reliability features.

## Features

- **Multiple Providers**: Email (SMTP), webhooks, Discord, Slack, Telegram, and more
- **Template Engine**: Handlebars-based templating for customizable notifications
- **Event-Driven**: Automatic notifications for movie events (downloaded, upgraded, failed)
- **Delivery Reliability**: Retry logic, circuit breakers, and failure tracking
- **Rich Content**: Support for images, embeds, and formatted messages
- **Filtering**: Configurable notification rules and conditions
- **Testing**: Built-in test message functionality for provider validation

## Key Dependencies

- **lettre**: SMTP email client with async support and TLS
- **reqwest**: HTTP client for webhook and API-based notifications
- **handlebars**: Template engine for message customization
- **serde/serde_json**: Configuration and message serialization
- **tokio**: Async runtime support for concurrent notifications
- **chrono**: Date/time handling for timestamps and scheduling
- **mime**: MIME type handling for email attachments

## Core Architecture

### Notification Service

```rust
use radarr_notifications::{NotificationService, NotificationConfig};

// Create service with multiple providers
let config = NotificationConfig {
    providers: vec![
        email_provider_config,
        discord_webhook_config,
        slack_webhook_config,
    ],
    global_templates: template_config,
    retry_config: RetryConfig::default(),
};

let service = NotificationService::new(config)?;

// Send notification for movie event
let movie_downloaded = MovieDownloadedEvent {
    movie_title: "The Matrix".to_string(),
    year: 1999,
    quality: "1080p BluRay".to_string(),
    size_mb: 8000,
    download_client: "qBittorrent".to_string(),
    downloaded_at: Utc::now(),
};

service.notify_movie_downloaded(&movie_downloaded).await?;
```

## Email Notifications

### SMTP Configuration

```rust
use radarr_notifications::{EmailProvider, EmailConfig};

let email_config = EmailConfig {
    smtp_server: "smtp.gmail.com".to_string(),
    smtp_port: 587,
    username: "your-email@gmail.com".to_string(),
    password: "app-specific-password".to_string(),
    use_tls: true,
    from_address: "radarr@yourdomain.com".to_string(),
    from_name: Some("Radarr".to_string()),
    to_addresses: vec![
        "admin@yourdomain.com".to_string(),
        "user@yourdomain.com".to_string(),
    ],
};

let email_provider = EmailProvider::new(email_config)?;

// Send custom email
let email_notification = EmailNotification {
    subject: "Movie Downloaded: {{movie_title}}".to_string(),
    body_text: Some("{{movie_title}} ({{year}}) has been downloaded in {{quality}} quality.".to_string()),
    body_html: Some(r#"
        <h2>Movie Downloaded</h2>
        <p><strong>{{movie_title}}</strong> ({{year}})</p>
        <p>Quality: {{quality}}</p>
        <p>Size: {{size_mb}} MB</p>
        <p>Downloaded at: {{downloaded_at}}</p>
    "#.to_string()),
    attachments: vec![],
};

email_provider.send(&email_notification, &context).await?;
```

### Email Templates

```rust
use radarr_notifications::{EmailTemplate, TemplateContext};

// HTML email template with styling
let html_template = r#"
<!DOCTYPE html>
<html>
<head>
    <style>
        .container { max-width: 600px; margin: 0 auto; font-family: Arial, sans-serif; }
        .header { background: #007acc; color: white; padding: 20px; text-align: center; }
        .content { padding: 20px; }
        .movie-info { background: #f5f5f5; padding: 15px; margin: 10px 0; border-radius: 5px; }
        .quality-badge { background: #28a745; color: white; padding: 3px 8px; border-radius: 3px; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🎬 Movie Downloaded</h1>
        </div>
        <div class="content">
            <div class="movie-info">
                <h2>{{movie_title}} ({{year}})</h2>
                <p><strong>Quality:</strong> <span class="quality-badge">{{quality}}</span></p>
                <p><strong>Size:</strong> {{size_mb}} MB</p>
                <p><strong>Downloaded:</strong> {{downloaded_at}}</p>
                {{#if poster_url}}
                <img src="{{poster_url}}" alt="Movie Poster" style="max-width: 200px;">
                {{/if}}
            </div>
        </div>
    </div>
</body>
</html>
"#;
```

## Webhook Notifications

### Generic Webhook Support

```rust
use radarr_notifications::{WebhookProvider, WebhookConfig};

let webhook_config = WebhookConfig {
    url: "https://api.yourservice.com/webhooks/radarr".to_string(),
    method: HttpMethod::Post,
    headers: vec![
        ("Authorization".to_string(), "Bearer your-token".to_string()),
        ("Content-Type".to_string(), "application/json".to_string()),
    ],
    timeout_seconds: 30,
    verify_ssl: true,
};

let webhook_provider = WebhookProvider::new(webhook_config)?;

// Send JSON payload
let payload = json!({
    "event": "movie.downloaded",
    "movie": {
        "title": "The Matrix",
        "year": 1999,
        "quality": "1080p BluRay"
    },
    "timestamp": Utc::now()
});

webhook_provider.send_json(&payload).await?;
```

## Discord Integration

### Discord Webhook with Embeds

```rust
use radarr_notifications::{DiscordProvider, DiscordConfig, DiscordEmbed};

let discord_config = DiscordConfig {
    webhook_url: "https://discord.com/api/webhooks/...".to_string(),
    username: Some("Radarr".to_string()),
    avatar_url: Some("https://radarr.video/logo.png".to_string()),
};

let discord_provider = DiscordProvider::new(discord_config)?;

// Rich embed notification
let embed = DiscordEmbed {
    title: Some("🎬 Movie Downloaded".to_string()),
    description: Some("A new movie has been added to your library".to_string()),
    color: Some(0x007acc), // Blue color
    fields: vec![
        DiscordField {
            name: "Title".to_string(),
            value: "The Matrix (1999)".to_string(),
            inline: true,
        },
        DiscordField {
            name: "Quality".to_string(),
            value: "1080p BluRay".to_string(),
            inline: true,
        },
        DiscordField {
            name: "Size".to_string(),
            value: "8.0 GB".to_string(),
            inline: true,
        },
    ],
    thumbnail: Some(DiscordThumbnail {
        url: "https://image.tmdb.org/t/p/w500/movie_poster.jpg".to_string(),
    }),
    timestamp: Some(Utc::now()),
    footer: Some(DiscordFooter {
        text: "Radarr Movie Automation".to_string(),
        icon_url: None,
    }),
};

discord_provider.send_embed(&embed).await?;
```

## Slack Integration

### Slack Webhook with Attachments

```rust
use radarr_notifications::{SlackProvider, SlackConfig, SlackAttachment};

let slack_config = SlackConfig {
    webhook_url: "https://hooks.slack.com/services/...".to_string(),
    channel: Some("#movies".to_string()),
    username: Some("Radarr".to_string()),
    icon_emoji: Some(":movie_camera:".to_string()),
};

let slack_provider = SlackProvider::new(slack_config)?;

let attachment = SlackAttachment {
    color: "good".to_string(), // green, warning, danger, or hex color
    title: Some("The Matrix (1999)".to_string()),
    title_link: Some("https://www.imdb.com/title/tt0133093/".to_string()),
    text: Some("Downloaded in 1080p BluRay quality (8.0 GB)".to_string()),
    fields: vec![
        SlackField {
            title: "Quality".to_string(),
            value: "1080p BluRay".to_string(),
            short: true,
        },
        SlackField {
            title: "Size".to_string(),
            value: "8.0 GB".to_string(),
            short: true,
        },
    ],
    thumb_url: Some("https://image.tmdb.org/t/p/w500/poster.jpg".to_string()),
    footer: "Radarr".to_string(),
    ts: Utc::now().timestamp(),
};

slack_provider.send_attachment(&attachment).await?;
```

## Event System Integration

### Automatic Event Notifications

```rust
use radarr_notifications::{NotificationService, MovieEvent};
use radarr_core::events::{EventBus, EventHandler};

// Event handler for automatic notifications
pub struct NotificationEventHandler {
    notification_service: NotificationService,
}

#[async_trait]
impl EventHandler<MovieDownloadedEvent> for NotificationEventHandler {
    async fn handle(&self, event: &MovieDownloadedEvent) -> Result<()> {
        self.notification_service.notify_movie_downloaded(event).await?;
        Ok(())
    }
}

#[async_trait]
impl EventHandler<MovieUpgradedEvent> for NotificationEventHandler {
    async fn handle(&self, event: &MovieUpgradedEvent) -> Result<()> {
        self.notification_service.notify_movie_upgraded(event).await?;
        Ok(())
    }
}

// Register with event bus
let event_bus = EventBus::new();
let notification_handler = NotificationEventHandler::new(notification_service);

event_bus.subscribe(notification_handler.clone()).await;
```

## Template System

### Custom Template Variables

```rust
use radarr_notifications::{TemplateEngine, TemplateContext};

let template_engine = TemplateEngine::new();

// Register custom helpers
template_engine.register_helper("format_size", |size_bytes: u64| -> String {
    if size_bytes > 1_000_000_000 {
        format!("{:.1} GB", size_bytes as f64 / 1_000_000_000.0)
    } else {
        format!("{:.1} MB", size_bytes as f64 / 1_000_000.0)
    }
});

template_engine.register_helper("time_ago", |datetime: DateTime<Utc>| -> String {
    let duration = Utc::now().signed_duration_since(datetime);
    if duration.num_hours() > 0 {
        format!("{} hours ago", duration.num_hours())
    } else {
        format!("{} minutes ago", duration.num_minutes())
    }
});

// Template with custom helpers
let template = r#"
{{movie_title}} ({{year}}) downloaded!
Size: {{format_size size_bytes}}
Downloaded: {{time_ago downloaded_at}}
"#;

let context = TemplateContext {
    movie_title: "The Matrix".to_string(),
    year: 1999,
    size_bytes: 8_000_000_000,
    downloaded_at: Utc::now(),
    // ... other fields
};

let rendered = template_engine.render(template, &context)?;
```

## Notification Models

### Core Event Types

```rust
use radarr_notifications::{MovieEvent, NotificationLevel};
use chrono::{DateTime, Utc};

// Movie downloaded event
pub struct MovieDownloadedEvent {
    pub movie_id: Uuid,
    pub movie_title: String,
    pub year: i32,
    pub quality: String,
    pub size_bytes: u64,
    pub download_client: String,
    pub downloaded_at: DateTime<Utc>,
    pub poster_url: Option<String>,
    pub imdb_url: Option<String>,
}

// Movie upgraded event  
pub struct MovieUpgradedEvent {
    pub movie_id: Uuid,
    pub movie_title: String,
    pub year: i32,
    pub old_quality: String,
    pub new_quality: String,
    pub old_size_bytes: u64,
    pub new_size_bytes: u64,
    pub upgraded_at: DateTime<Utc>,
}

// Movie failed event
pub struct MovieFailedEvent {
    pub movie_id: Uuid,
    pub movie_title: String,
    pub year: i32,
    pub error_message: String,
    pub failed_at: DateTime<Utc>,
    pub level: NotificationLevel, // Info, Warning, Error, Critical
}
```

## Configuration

### Comprehensive Configuration

```rust
use radarr_notifications::NotificationConfig;

let config = NotificationConfig {
    providers: vec![
        ProviderConfig::Email(EmailConfig {
            smtp_server: "smtp.gmail.com".to_string(),
            smtp_port: 587,
            username: env::var("EMAIL_USERNAME")?,
            password: env::var("EMAIL_PASSWORD")?,
            use_tls: true,
            from_address: "radarr@yourdomain.com".to_string(),
            to_addresses: vec!["admin@yourdomain.com".to_string()],
        }),
        ProviderConfig::Discord(DiscordConfig {
            webhook_url: env::var("DISCORD_WEBHOOK_URL")?,
            username: Some("Radarr".to_string()),
        }),
        ProviderConfig::Webhook(WebhookConfig {
            url: "https://api.pushover.net/1/messages.json".to_string(),
            method: HttpMethod::Post,
            headers: vec![
                ("Content-Type".to_string(), "application/json".to_string()),
            ],
            timeout_seconds: 30,
        }),
    ],
    event_filters: vec![
        EventFilter {
            event_types: vec![EventType::MovieDownloaded, EventType::MovieUpgraded],
            quality_filter: Some(QualityFilter::MinimumQuality("720p".to_string())),
            size_filter: Some(SizeFilter::MinimumSize(500_000_000)), // 500MB
        },
    ],
    retry_config: RetryConfig {
        max_retries: 3,
        initial_delay: Duration::from_secs(5),
        backoff_multiplier: 2.0,
        max_delay: Duration::from_secs(300),
    },
    global_templates: TemplateConfig {
        movie_downloaded_subject: "🎬 {{movie_title}} Downloaded".to_string(),
        movie_downloaded_body: "{{movie_title}} ({{year}}) has been downloaded in {{quality}} quality.".to_string(),
        // ... other templates
    },
};
```

## Testing

### Provider Testing

```rust
use radarr_notifications::{NotificationService, TestNotification};

// Test all configured providers
let test_results = notification_service.test_all_providers().await?;

for result in test_results {
    match result {
        Ok(provider_name) => println!("✓ {} - Test successful", provider_name),
        Err((provider_name, error)) => println!("✗ {} - Test failed: {}", provider_name, error),
    }
}

// Send test notification
let test_notification = TestNotification {
    title: "Radarr Test Notification".to_string(),
    message: "This is a test message to verify notification delivery.".to_string(),
    timestamp: Utc::now(),
};

notification_service.send_test_notification(&test_notification).await?;
```

### Mock Providers for Testing

```rust
use radarr_notifications::MockNotificationProvider;
use wiremock::{MockServer, Mock, ResponseTemplate};

#[tokio::test]
async fn test_webhook_notification() {
    let mock_server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/webhook"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;
    
    let webhook_config = WebhookConfig {
        url: format!("{}/webhook", mock_server.uri()),
        // ... other config
    };
    
    let provider = WebhookProvider::new(webhook_config)?;
    let result = provider.send_json(&test_payload).await;
    
    assert!(result.is_ok());
}
```

## Error Handling and Reliability

### Delivery Guarantees

```rust
use radarr_notifications::{DeliveryResult, RetryPolicy};

// Automatic retry with backoff
let delivery_result = notification_service
    .send_with_retry(&notification, RetryPolicy::ExponentialBackoff)
    .await;

match delivery_result {
    DeliveryResult::Success { provider, duration } => {
        println!("Notification delivered via {} in {}ms", provider, duration.as_millis());
    }
    DeliveryResult::PartialSuccess { successful, failed } => {
        println!("Delivered to {} providers, failed on {} providers", 
                 successful.len(), failed.len());
    }
    DeliveryResult::Failed { errors } => {
        println!("All delivery attempts failed:");
        for (provider, error) in errors {
            println!("  {}: {}", provider, error);
        }
    }
}
```

### Circuit Breaker Pattern

```rust
use radarr_notifications::CircuitBreakerConfig;

let circuit_breaker_config = CircuitBreakerConfig {
    failure_threshold: 5,           // Open after 5 failures
    recovery_timeout: Duration::from_secs(300), // 5 minute recovery period
    half_open_max_calls: 3,         // Test with 3 calls when half-open
};

// Circuit breaker automatically handles failing providers
// Temporarily disables providers that are consistently failing
// Automatically re-enables them after recovery period
```

## Integration with Core System

```rust
use radarr_core::{Movie, Download, DownloadStatus};
use radarr_notifications::{NotificationService, MovieDownloadedEvent};

async fn handle_download_completion(
    download: &Download,
    movie: &Movie,
    notification_service: &NotificationService,
) -> Result<()> {
    if download.status == DownloadStatus::Completed {
        let event = MovieDownloadedEvent {
            movie_id: movie.id,
            movie_title: movie.title.clone(),
            year: movie.year,
            quality: download.quality.to_string(),
            size_bytes: download.size_bytes,
            download_client: download.download_client.clone(),
            downloaded_at: download.completed_at.unwrap_or(Utc::now()),
            poster_url: movie.poster_url.clone(),
            imdb_url: movie.imdb_url.clone(),
        };
        
        notification_service.notify_movie_downloaded(&event).await?;
    }
    
    Ok(())
}
```