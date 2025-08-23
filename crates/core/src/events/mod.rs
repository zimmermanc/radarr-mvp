//! Event system for inter-component communication
//!
//! This module provides a simple event bus using tokio broadcast channels
//! to enable loose coupling between components like downloads, imports, and notifications.

use crate::{Result, RadarrError};
use crate::correlation::{CorrelationId, CorrelationContext, current_correlation_id};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Maximum number of events to buffer in the channel
const EVENT_BUFFER_SIZE: usize = 1000;

/// Event envelope that includes correlation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    /// Unique ID for this event instance
    pub event_id: Uuid,
    
    /// Correlation ID for tracking across services
    pub correlation_id: CorrelationId,
    
    /// Timestamp when the event was created
    pub timestamp: chrono::DateTime<chrono::Utc>,
    
    /// The actual event data
    pub event: SystemEvent,
    
    /// Optional source component/service
    pub source: Option<String>,
}

impl EventEnvelope {
    /// Create a new event envelope with the current correlation context
    pub fn new(event: SystemEvent) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            correlation_id: current_correlation_id(),
            timestamp: chrono::Utc::now(),
            event,
            source: None,
        }
    }
    
    /// Create a new event envelope with a specific correlation ID
    pub fn with_correlation(event: SystemEvent, correlation_id: CorrelationId) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            correlation_id,
            timestamp: chrono::Utc::now(),
            event,
            source: None,
        }
    }
    
    /// Set the source component
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }
    
    /// Get a description including correlation info
    pub fn description(&self) -> String {
        format!(
            "[{}] {} (event_id={})",
            self.correlation_id,
            self.event.description(),
            self.event_id
        )
    }
}

/// System events that can be published and subscribed to
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum SystemEvent {
    /// Download was added to the queue
    DownloadQueued {
        movie_id: Uuid,
        release_id: Uuid,
        download_url: String,
        title: String,
    },
    /// Download started
    DownloadStarted {
        movie_id: Uuid,
        queue_item_id: Uuid,
        client_id: String,
    },
    /// Download progress update
    DownloadProgress {
        movie_id: Uuid,
        queue_item_id: Uuid,
        progress: f64,
        speed: Option<u64>,
        eta_seconds: Option<u64>,
    },
    /// Download completed successfully
    DownloadComplete {
        movie_id: Uuid,
        queue_item_id: Uuid,
        file_path: String,
    },
    /// Download failed
    DownloadFailed {
        movie_id: Uuid,
        queue_item_id: Uuid,
        error: String,
    },
    /// Import process started
    ImportTriggered {
        movie_id: Uuid,
        source_path: String,
    },
    /// Import completed successfully
    ImportComplete {
        movie_id: Uuid,
        destination_path: String,
        file_count: usize,
    },
    /// Import failed
    ImportFailed {
        movie_id: Uuid,
        source_path: String,
        error: String,
    },
    /// Movie metadata updated
    MovieUpdated {
        movie_id: Uuid,
        changes: Vec<String>,
    },
    /// Quality profile changed
    QualityProfileUpdated {
        profile_id: Uuid,
        name: String,
    },
    /// System health event
    SystemHealth {
        component: String,
        status: String,
        message: Option<String>,
    },
    /// Progress update for any operation
    ProgressUpdate {
        operation_id: Uuid,
        operation_type: crate::progress::OperationType,
        percentage: f32,
        message: String,
        eta_seconds: Option<u64>,
    },
    /// Operation completed
    OperationComplete {
        operation_id: Uuid,
        operation_type: crate::progress::OperationType,
        success: bool,
        message: String,
    },
}

impl SystemEvent {
    /// Get the movie ID associated with this event, if any
    pub fn movie_id(&self) -> Option<Uuid> {
        match self {
            SystemEvent::DownloadQueued { movie_id, .. }
            | SystemEvent::DownloadStarted { movie_id, .. }
            | SystemEvent::DownloadProgress { movie_id, .. }
            | SystemEvent::DownloadComplete { movie_id, .. }
            | SystemEvent::DownloadFailed { movie_id, .. }
            | SystemEvent::ImportTriggered { movie_id, .. }
            | SystemEvent::ImportComplete { movie_id, .. }
            | SystemEvent::ImportFailed { movie_id, .. }
            | SystemEvent::MovieUpdated { movie_id, .. } => Some(*movie_id),
            _ => None,
        }
    }

    /// Get a short description of the event for logging
    pub fn description(&self) -> String {
        match self {
            SystemEvent::DownloadQueued { title, .. } => format!("Download queued: {}", title),
            SystemEvent::DownloadStarted { client_id, .. } => format!("Download started: {}", client_id),
            SystemEvent::DownloadProgress { progress, .. } => format!("Download progress: {:.1}%", progress * 100.0),
            SystemEvent::DownloadComplete { file_path, .. } => format!("Download complete: {}", file_path),
            SystemEvent::DownloadFailed { error, .. } => format!("Download failed: {}", error),
            SystemEvent::ImportTriggered { source_path, .. } => format!("Import triggered: {}", source_path),
            SystemEvent::ImportComplete { destination_path, file_count, .. } => {
                format!("Import complete: {} files to {}", file_count, destination_path)
            }
            SystemEvent::ImportFailed { error, .. } => format!("Import failed: {}", error),
            SystemEvent::MovieUpdated { changes, .. } => format!("Movie updated: {}", changes.join(", ")),
            SystemEvent::QualityProfileUpdated { name, .. } => format!("Quality profile updated: {}", name),
            SystemEvent::SystemHealth { component, status, .. } => format!("Health: {} is {}", component, status),
            SystemEvent::ProgressUpdate { percentage, message, .. } => format!("Progress: {:.1}% - {}", percentage, message),
            SystemEvent::OperationComplete { success, message, .. } => format!("Operation {}: {}", if *success { "completed" } else { "failed" }, message),
        }
    }
}

/// Event bus for publishing and subscribing to system events
#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<EventEnvelope>,
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(EVENT_BUFFER_SIZE);
        Self { sender }
    }

    /// Publish an event to all subscribers
    pub async fn publish(&self, event: SystemEvent) -> Result<()> {
        let envelope = EventEnvelope::new(event);
        self.publish_envelope(envelope).await
    }
    
    /// Publish an event with a specific correlation ID
    pub async fn publish_with_correlation(&self, event: SystemEvent, correlation_id: CorrelationId) -> Result<()> {
        let envelope = EventEnvelope::with_correlation(event, correlation_id);
        self.publish_envelope(envelope).await
    }
    
    /// Publish an event envelope
    pub async fn publish_envelope(&self, envelope: EventEnvelope) -> Result<()> {
        debug!("Publishing event: {}", envelope.description());
        
        match self.sender.send(envelope.clone()) {
            Ok(receiver_count) => {
                if receiver_count > 0 {
                    debug!("Event published to {} receivers", receiver_count);
                } else {
                    debug!("Event published but no receivers");
                }
                Ok(())
            }
            Err(broadcast::error::SendError(_)) => {
                warn!("Failed to publish event - no receivers");
                Ok(()) // Not really an error if no one is listening
            }
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> EventSubscriber {
        let receiver = self.sender.subscribe();
        EventSubscriber { receiver }
    }

    /// Get the number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Event subscriber that can receive events
pub struct EventSubscriber {
    receiver: broadcast::Receiver<EventEnvelope>,
}

impl EventSubscriber {
    /// Receive the next event envelope (blocking)
    pub async fn recv(&mut self) -> Result<EventEnvelope> {
        match self.receiver.recv().await {
            Ok(envelope) => {
                debug!("Received event: {}", envelope.description());
                // Set the correlation context for this thread
                if let Some(ctx) = crate::correlation::current_context() {
                    if ctx.correlation_id != envelope.correlation_id {
                        // Update to use the event's correlation ID
                        let new_ctx = CorrelationContext::new("event_handler")
                            .with_session(ctx.session_id.unwrap_or_default())
                            .with_user(ctx.user_id.unwrap_or_default());
                        crate::correlation::set_current_context(new_ctx);
                    }
                } else {
                    // Set a new context with the event's correlation ID
                    let ctx = CorrelationContext::new("event_handler");
                    crate::correlation::set_current_context(ctx);
                }
                Ok(envelope)
            }
            Err(broadcast::error::RecvError::Closed) => {
                Err(RadarrError::ExternalServiceError {
                    service: "event_bus".to_string(),
                    error: "Event bus channel closed".to_string(),
                })
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                warn!("Event subscriber lagged, skipped {} events", skipped);
                // Continue receiving - use Box::pin to avoid infinite future
                Box::pin(self.recv()).await
            }
        }
    }
    
    /// Receive just the event (for backward compatibility)
    pub async fn recv_event(&mut self) -> Result<SystemEvent> {
        let envelope = self.recv().await?;
        Ok(envelope.event)
    }

    /// Try to receive an event envelope without blocking
    pub fn try_recv(&mut self) -> Result<Option<EventEnvelope>> {
        match self.receiver.try_recv() {
            Ok(envelope) => {
                debug!("Received event (non-blocking): {}", envelope.description());
                Ok(Some(envelope))
            }
            Err(broadcast::error::TryRecvError::Empty) => Ok(None),
            Err(broadcast::error::TryRecvError::Closed) => {
                Err(RadarrError::ExternalServiceError {
                    service: "event_bus".to_string(),
                    error: "Event bus channel closed".to_string(),
                })
            }
            Err(broadcast::error::TryRecvError::Lagged(skipped)) => {
                warn!("Event subscriber lagged, skipped {} events", skipped);
                Ok(None)
            }
        }
    }
    
    /// Try to receive just the event without blocking (for backward compatibility)
    pub fn try_recv_event(&mut self) -> Result<Option<SystemEvent>> {
        Ok(self.try_recv()?.map(|envelope| envelope.event))
    }
}

/// Event handler trait for components that want to process events
#[async_trait::async_trait]
pub trait EventHandler: Send + Sync {
    /// Handle an incoming event envelope
    async fn handle_event(&self, envelope: &EventEnvelope) -> Result<()>;

    /// Filter events - return true to process, false to ignore
    fn should_handle(&self, envelope: &EventEnvelope) -> bool {
        // By default, handle all events
        let _ = envelope;
        true
    }
}

/// Event processor that runs in the background and forwards events to handlers
pub struct EventProcessor {
    subscriber: EventSubscriber,
    handlers: Vec<Arc<dyn EventHandler>>,
}

impl EventProcessor {
    /// Create a new event processor
    pub fn new(event_bus: &EventBus) -> Self {
        Self {
            subscriber: event_bus.subscribe(),
            handlers: Vec::new(),
        }
    }

    /// Add an event handler
    pub fn add_handler(mut self, handler: Arc<dyn EventHandler>) -> Self {
        self.handlers.push(handler);
        self
    }

    /// Start processing events (runs until the event bus is closed)
    pub async fn run(mut self) -> Result<()> {
        info!("Starting event processor with {} handlers", self.handlers.len());

        loop {
            match self.subscriber.recv().await {
                Ok(envelope) => {
                    debug!("Processing event: {} with correlation_id={}", 
                           envelope.description(), 
                           envelope.correlation_id);
                    
                    // Set correlation context for this processing
                    let ctx = CorrelationContext::new("event_processor");
                    crate::correlation::set_current_context(ctx);
                    
                    // Process event with all interested handlers
                    for handler in &self.handlers {
                        if handler.should_handle(&envelope) {
                            if let Err(e) = handler.handle_event(&envelope).await {
                                error!("Handler failed to process event {} with correlation_id={}: {}", 
                                       envelope.description(), 
                                       envelope.correlation_id, 
                                       e);
                                // Continue with other handlers
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Event processor error: {}", e);
                    break;
                }
            }
        }

        info!("Event processor shutting down");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::time::{timeout, Duration};

    struct TestHandler {
        counter: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl EventHandler for TestHandler {
        async fn handle_event(&self, _envelope: &EventEnvelope) -> Result<()> {
            self.counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_event_bus_basic() {
        let event_bus = EventBus::new();
        let mut subscriber = event_bus.subscribe();

        let event = SystemEvent::DownloadQueued {
            movie_id: Uuid::new_v4(),
            release_id: Uuid::new_v4(),
            download_url: "magnet:test".to_string(),
            title: "Test Movie".to_string(),
        };

        // Publish event
        event_bus.publish(event.clone()).await.unwrap();

        // Receive event
        let envelope = subscriber.recv().await.unwrap();
        
        if let SystemEvent::DownloadQueued { title, .. } = envelope.event {
            assert_eq!(title, "Test Movie");
        } else {
            panic!("Wrong event type received");
        }
        
        // Check that correlation ID was set
        assert_ne!(envelope.correlation_id, CorrelationId::from_uuid(Uuid::nil()));
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let event_bus = EventBus::new();
        let mut sub1 = event_bus.subscribe();
        let mut sub2 = event_bus.subscribe();

        assert_eq!(event_bus.subscriber_count(), 2);

        let event = SystemEvent::SystemHealth {
            component: "test".to_string(),
            status: "healthy".to_string(),
            message: None,
        };

        event_bus.publish(event.clone()).await.unwrap();

        // Both subscribers should receive the event
        let recv1 = timeout(Duration::from_millis(100), sub1.recv()).await.unwrap().unwrap();
        let recv2 = timeout(Duration::from_millis(100), sub2.recv()).await.unwrap().unwrap();

        assert!(matches!(recv1.event, SystemEvent::SystemHealth { .. }));
        assert!(matches!(recv2.event, SystemEvent::SystemHealth { .. }));
        
        // Both should have the same correlation ID
        assert_eq!(recv1.correlation_id, recv2.correlation_id);
    }

    #[tokio::test]
    async fn test_event_processor() {
        let event_bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        
        let handler = Arc::new(TestHandler {
            counter: counter.clone(),
        });

        let processor = EventProcessor::new(&event_bus)
            .add_handler(handler);

        // Start processor in background
        let processor_handle = tokio::spawn(async move {
            processor.run().await
        });

        // Give processor time to start
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Publish some events
        for i in 0..5 {
            let event = SystemEvent::MovieUpdated {
                movie_id: Uuid::new_v4(),
                changes: vec![format!("change_{}", i)],
            };
            event_bus.publish(event).await.unwrap();
        }

        // Give processor time to handle events
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Check that all events were processed
        assert_eq!(counter.load(Ordering::SeqCst), 5);

        // Stop the processor by dropping the event bus
        drop(event_bus);
        
        // Processor should shut down
        let _ = timeout(Duration::from_millis(100), processor_handle).await;
    }

    #[test]
    fn test_event_movie_id_extraction() {
        let movie_id = Uuid::new_v4();
        
        let event = SystemEvent::DownloadComplete {
            movie_id,
            queue_item_id: Uuid::new_v4(),
            file_path: "/downloads/movie.mkv".to_string(),
        };

        assert_eq!(event.movie_id(), Some(movie_id));

        let health_event = SystemEvent::SystemHealth {
            component: "test".to_string(),
            status: "healthy".to_string(),
            message: None,
        };

        assert_eq!(health_event.movie_id(), None);
    }

    #[tokio::test]
    async fn test_correlation_id_propagation() {
        let event_bus = EventBus::new();
        let mut subscriber = event_bus.subscribe();
        
        // Set a specific correlation ID
        let correlation_id = CorrelationId::new();
        let event = SystemEvent::ImportTriggered {
            movie_id: Uuid::new_v4(),
            source_path: "/downloads/movie.mkv".to_string(),
        };
        
        // Publish with specific correlation ID
        event_bus.publish_with_correlation(event, correlation_id).await.unwrap();
        
        // Receive and verify
        let envelope = subscriber.recv().await.unwrap();
        assert_eq!(envelope.correlation_id, correlation_id);
        
        // Publish another event without setting correlation ID
        let event2 = SystemEvent::ImportComplete {
            movie_id: Uuid::new_v4(),
            destination_path: "/media/movies/movie.mkv".to_string(),
            file_count: 1,
        };
        
        // Set correlation context to simulate being in the same operation
        crate::correlation::set_current_context(
            CorrelationContext::new("test").with_session("test_session")
        );
        
        event_bus.publish(event2).await.unwrap();
        
        let envelope2 = subscriber.recv().await.unwrap();
        // Should have a different correlation ID since we didn't propagate it
        assert_ne!(envelope2.correlation_id, correlation_id);
        
        crate::correlation::clear_context();
    }

    #[test]
    fn test_event_descriptions() {
        let event = SystemEvent::DownloadProgress {
            movie_id: Uuid::new_v4(),
            queue_item_id: Uuid::new_v4(),
            progress: 0.75,
            speed: Some(1024 * 1024),
            eta_seconds: Some(300),
        };

        let desc = event.description();
        assert!(desc.contains("75.0%"));
        assert!(desc.contains("Download progress"));
    }
}