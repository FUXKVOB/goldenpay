use crate::client::GoldenPaySession;
use crate::error::GoldenPayError;
use crate::models::{BotState, ChatMessage, OrderInfo};
use crate::storage::{JsonStateStore, MemoryStateStore, StateStore};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Polling events emitted by [`GoldenPayBot`].
#[derive(Debug, Clone)]
pub enum GoldenPayEvent {
    NewOrder(OrderInfo),
    NewMessage(ChatMessage),
}

/// High-level polling bot with pluggable persistent state storage.
pub struct GoldenPayBot {
    session: GoldenPaySession,
    store: Arc<dyn StateStore>,
    seen_orders: HashSet<String>,
    seen_messages: HashMap<String, i64>,
}

impl GoldenPayBot {
    /// Creates a bot and automatically selects JSON or memory state storage
    /// based on the session configuration.
    pub fn new(session: GoldenPaySession) -> Self {
        let store: Arc<dyn StateStore> = if let Some(path) = session.config().state_path.clone() {
            Arc::new(JsonStateStore::new(path))
        } else {
            Arc::new(MemoryStateStore::new())
        };

        Self {
            session,
            store,
            seen_orders: HashSet::new(),
            seen_messages: HashMap::new(),
        }
    }

    /// Creates a bot with an explicit custom state store.
    pub fn with_store(session: GoldenPaySession, store: Arc<dyn StateStore>) -> Self {
        Self {
            session,
            store,
            seen_orders: HashSet::new(),
            seen_messages: HashMap::new(),
        }
    }

    /// Returns the underlying authenticated session.
    pub fn session(&self) -> &GoldenPaySession {
        &self.session
    }

    /// Loads persisted bot state from storage.
    pub async fn load_state(&mut self) -> Result<(), GoldenPayError> {
        let state = self.store.load().await?;
        self.seen_orders = state.seen_orders.into_iter().collect();
        self.seen_messages = state.seen_messages;
        Ok(())
    }

    /// Saves current bot state to storage.
    pub async fn save_state(&self) -> Result<(), GoldenPayError> {
        self.store
            .save(&BotState {
                seen_orders: self.seen_orders.iter().cloned().collect(),
                seen_messages: self.seen_messages.clone(),
            })
            .await
    }

    /// Initializes the bot from the current remote snapshot without emitting events.
    pub async fn bootstrap(&mut self) -> Result<(), GoldenPayError> {
        let orders = self.session.fetch_orders().await?;
        for order in orders {
            let chat_id = order.chat_id.clone();
            self.seen_orders.insert(order.id);

            let mut last_message_id = 0;
            for message in self.session.fetch_chat_messages(&chat_id).await? {
                last_message_id = last_message_id.max(message.id);
            }

            if last_message_id > 0 {
                self.seen_messages.insert(chat_id, last_message_id);
            }
        }

        self.save_state().await
    }

    /// Performs one polling iteration and returns newly discovered events.
    pub async fn poll_once(&mut self) -> Result<Vec<GoldenPayEvent>, GoldenPayError> {
        let orders = self.session.fetch_orders().await?;
        let mut events = Vec::new();

        for order in orders {
            let chat_id = order.chat_id.clone();
            let is_new_order = self.seen_orders.insert(order.id.clone());

            if is_new_order {
                events.push(GoldenPayEvent::NewOrder(order.clone()));
            }

            let mut last_seen = self
                .seen_messages
                .get(&chat_id)
                .copied()
                .unwrap_or_default();
            for message in self.session.fetch_chat_messages(&chat_id).await? {
                if message.id > last_seen {
                    last_seen = message.id;
                    events.push(GoldenPayEvent::NewMessage(message));
                }
            }

            if last_seen > 0 {
                self.seen_messages.insert(chat_id, last_seen);
            }
        }

        self.save_state().await?;
        Ok(events)
    }

    /// Runs the bot forever and forwards each event into the provided async handler.
    pub async fn run<F, Fut>(&mut self, mut handler: F) -> Result<(), GoldenPayError>
    where
        F: FnMut(GoldenPayEvent, &GoldenPaySession) -> Fut,
        Fut: std::future::Future<Output = Result<(), GoldenPayError>>,
    {
        loop {
            for event in self.poll_once().await? {
                handler(event, &self.session).await?;
            }

            tokio::time::sleep(self.session.poll_interval()).await;
        }
    }
}
