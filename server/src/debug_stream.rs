use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

#[derive(Debug, Clone)]
pub struct DebugMessage {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub source: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
    Debug,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warning => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Debug => write!(f, "DEBUG"),
        }
    }
}

/// Global debug stream service that can be shared between server and TUI
pub struct DebugStream {
    sender: broadcast::Sender<DebugMessage>,
    max_messages: usize,
    message_history: Arc<RwLock<Vec<DebugMessage>>>,
}

impl DebugStream {
    pub fn new(max_messages: usize) -> Self {
        let (sender, _) = broadcast::channel(max_messages); // Buffer up to 1000 messages

        Self {
            sender,
            max_messages,
            message_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Send a debug message to all subscribers
    pub async fn send(&self, level: LogLevel, source: &str, message: &str) {
        let debug_msg = DebugMessage {
            timestamp: Utc::now(),
            level,
            source: source.to_string(),
            message: message.to_string(),
        };

        // Store in history
        {
            let mut history = self.message_history.write().await;
            history.push(debug_msg.clone());

            // Keep only the last max_messages
            if history.len() > self.max_messages {
                let excess = history.len() - self.max_messages;
                history.drain(0..excess);
            }
        }

        // Broadcast to subscribers
        let _ = self.sender.send(debug_msg);
    }

    /// Subscribe to debug messages
    pub fn subscribe(&self) -> broadcast::Receiver<DebugMessage> {
        self.sender.subscribe()
    }

    /// Get the message history
    pub async fn get_history(&self) -> Vec<DebugMessage> {
        self.message_history.read().await.clone()
    }

    /// Get recent messages (last N messages)
    pub async fn get_recent(&self, count: usize) -> Vec<DebugMessage> {
        let history = self.message_history.read().await;
        let start = if history.len() > count {
            history.len() - count
        } else {
            0
        };
        history[start..].to_vec()
    }

    /// Clear message history
    pub async fn clear_history(&self) {
        self.message_history.write().await.clear();
    }
}

/// Convenience methods for different log levels
impl DebugStream {
    pub async fn info(&self, source: &str, message: &str) {
        self.send(LogLevel::Info, source, message).await;
    }

    pub async fn warn(&self, source: &str, message: &str) {
        self.send(LogLevel::Warning, source, message).await;
    }

    pub async fn error(&self, source: &str, message: &str) {
        self.send(LogLevel::Error, source, message).await;
    }

    pub async fn debug(&self, source: &str, message: &str) {
        self.send(LogLevel::Debug, source, message).await;
    }
}
