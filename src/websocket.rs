//! WebSocket handler for real-time progress updates

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Extension,
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use radarr_core::{
    events::{EventBus, SystemEvent},
    progress::{ProgressTracker, OperationType},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
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

/// Handle WebSocket upgrade request
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Extension(state): Extension<Arc<WsState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
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
    
    // Spawn task to handle incoming messages
    let state_clone = state.clone();
    let mut sender_clone = sender.clone();
    let incoming_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                        handle_message(ws_msg, &state_clone, &mut sender_clone, &mut subscribed_operations).await;
                    }
                }
                Ok(Message::Ping(data)) => {
                    let _ = sender_clone.send(Message::Pong(data)).await;
                }
                Ok(Message::Close(_)) => {
                    info!("WebSocket client disconnected");
                    break;
                }
                _ => {}
            }
        }
    });
    
    // Forward events to WebSocket
    let event_task = tokio::spawn(async move {
        while let Ok(event) = event_receiver.recv().await {
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
    });
    
    // Wait for either task to complete
    tokio::select! {
        _ = incoming_task => {
            debug!("Incoming message handler completed");
        }
        _ = event_task => {
            debug!("Event forwarding task completed");
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