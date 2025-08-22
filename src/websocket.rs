//! WebSocket handler for real-time progress updates

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Extension, Query,
    },
    response::Response,
    http::StatusCode,
};
use futures::{sink::SinkExt, stream::StreamExt};
use radarr_core::{
    events::{EventBus, SystemEvent},
    progress::{ProgressTracker, OperationType},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsMessage {
    /// Subscribe to specific operation types
    Subscribe {
        operations: Vec<OperationType>,
    },
    /// Unsubscribe from operation types
    Unsubscribe {
        operations: Vec<OperationType>,
    },
    /// Request current progress for all operations
    GetProgress,
    /// Ping to keep connection alive
    Ping,
    /// Pong response
    Pong,
}

/// WebSocket response messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsResponse {
    /// Progress update
    Progress {
        operation_id: uuid::Uuid,
        operation_type: OperationType,
        percentage: f32,
        message: String,
        eta_seconds: Option<u64>,
        status: String,
    },
    /// Operation completed
    Complete {
        operation_id: uuid::Uuid,
        operation_type: OperationType,
        success: bool,
        message: String,
    },
    /// System event
    Event {
        event: SystemEvent,
    },
    /// Error message
    Error {
        message: String,
    },
    /// Pong response
    Pong,
    /// Current progress snapshot
    ProgressSnapshot {
        operations: Vec<radarr_core::progress::ProgressInfo>,
    },
}

/// WebSocket state
pub struct WsState {
    pub event_bus: Arc<EventBus>,
    pub progress_tracker: Arc<ProgressTracker>,
}

/// WebSocket query parameters for authentication
#[derive(Debug, Deserialize)]
pub struct WsQuery {
    pub apikey: Option<String>,
}

/// Handle WebSocket upgrade request with authentication
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsQuery>,
    Extension(state): Extension<Arc<WsState>>,
) -> Result<Response, StatusCode> {
    // Verify API key
    let expected_api_key = std::env::var("RADARR_API_KEY")
        .map_err(|_| {
            error!("RADARR_API_KEY environment variable not set");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    match params.apikey {
        Some(provided_key) if provided_key == expected_api_key => {
            info!("WebSocket client authenticated successfully");
            Ok(ws.on_upgrade(|socket| handle_socket(socket, state)))
        }
        Some(_) => {
            warn!("WebSocket authentication failed: invalid API key");
            Err(StatusCode::UNAUTHORIZED)
        }
        None => {
            warn!("WebSocket authentication failed: no API key provided");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

/// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, state: Arc<WsState>) {
    let (mut sender, mut receiver) = socket.split();
    
    // Subscribe to event bus
    let mut event_receiver = state.event_bus.subscribe();
    
    // Track subscribed operation types
    let mut subscribed_operations: Vec<OperationType> = vec![
        OperationType::Download,
        OperationType::Import,
        OperationType::IndexerSearch,
    ]; // Default subscriptions
    
    info!("WebSocket client connected");
    
    // Send initial progress snapshot
    let operations = state.progress_tracker.get_active_operations().await;
    let snapshot = WsResponse::ProgressSnapshot { operations };
    if let Ok(msg) = serde_json::to_string(&snapshot) {
        let _ = sender.send(Message::Text(msg)).await;
    }
    
    // Handle WebSocket communication in a single task
    loop {
        tokio::select! {
            // Handle incoming WebSocket messages
            ws_msg = receiver.next() => {
                match ws_msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                            handle_message(ws_msg, &state, &mut sender, &mut subscribed_operations).await;
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = sender.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("WebSocket client disconnected");
                        break;
                    }
                    Some(Err(_)) => {
                        break; // Connection error
                    }
                    None => {
                        break; // Stream ended
                    }
                    _ => {}
                }
            }
            // Handle events from event bus
            event = event_receiver.recv() => {
                match event {
                    Ok(event) => {
                        // Convert event to WsResponse
                        let response = match &event {
                            SystemEvent::ProgressUpdate { 
                                operation_id, 
                                operation_type, 
                                percentage, 
                                message, 
                                eta_seconds 
                            } => {
                                // Check if subscribed to this operation type
                                if subscribed_operations.contains(operation_type) {
                                    Some(WsResponse::Progress {
                                        operation_id: *operation_id,
                                        operation_type: *operation_type,
                                        percentage: *percentage,
                                        message: message.clone(),
                                        eta_seconds: *eta_seconds,
                                        status: "in_progress".to_string(),
                                    })
                                } else {
                                    None
                                }
                            }
                            SystemEvent::OperationComplete { 
                                operation_id, 
                                operation_type, 
                                success, 
                                message 
                            } => {
                                if subscribed_operations.contains(operation_type) {
                                    Some(WsResponse::Complete {
                                        operation_id: *operation_id,
                                        operation_type: *operation_type,
                                        success: *success,
                                        message: message.clone(),
                                    })
                                } else {
                                    None
                                }
                            }
                            _ => {
                                // Forward all other events
                                Some(WsResponse::Event { event: event.clone() })
                            }
                        };
                        
                        // Send response if we have one
                        if let Some(response) = response {
                            if let Ok(msg) = serde_json::to_string(&response) {
                                if sender.send(Message::Text(msg)).await.is_err() {
                                    break; // Client disconnected
                                }
                            }
                        }
                    }
                    Err(_) => {
                        break; // Event bus error
                    }
                }
            }
        }
    }
    
    info!("WebSocket handler shutting down");
}

/// Handle incoming WebSocket message
async fn handle_message(
    msg: WsMessage,
    state: &Arc<WsState>,
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    subscribed_operations: &mut Vec<OperationType>,
) {
    match msg {
        WsMessage::Subscribe { operations } => {
            for op in operations {
                if !subscribed_operations.contains(&op) {
                    subscribed_operations.push(op);
                }
            }
            debug!("Subscribed to operations: {:?}", subscribed_operations);
        }
        WsMessage::Unsubscribe { operations } => {
            subscribed_operations.retain(|op| !operations.contains(op));
            debug!("Unsubscribed from operations, now subscribed to: {:?}", subscribed_operations);
        }
        WsMessage::GetProgress => {
            let operations = state.progress_tracker.get_all_operations().await;
            let response = WsResponse::ProgressSnapshot { operations };
            if let Ok(msg) = serde_json::to_string(&response) {
                let _ = sender.send(Message::Text(msg)).await;
            }
        }
        WsMessage::Ping => {
            let response = WsResponse::Pong;
            if let Ok(msg) = serde_json::to_string(&response) {
                let _ = sender.send(Message::Text(msg)).await;
            }
        }
        WsMessage::Pong => {
            // Client sent pong, no action needed
        }
    }
}