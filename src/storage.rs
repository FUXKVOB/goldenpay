use crate::error::GoldenPayError;
use crate::models::BotState;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Mutex;

#[async_trait]
pub trait StateStore: Send + Sync {
    async fn load(&self) -> Result<BotState, GoldenPayError>;
    async fn save(&self, state: &BotState) -> Result<(), GoldenPayError>;
}

#[derive(Default)]
pub struct MemoryStateStore {
    state: Arc<Mutex<BotState>>,
}

impl MemoryStateStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl StateStore for MemoryStateStore {
    async fn load(&self) -> Result<BotState, GoldenPayError> {
        Ok(self.state.lock().await.clone())
    }

    async fn save(&self, state: &BotState) -> Result<(), GoldenPayError> {
        *self.state.lock().await = state.clone();
        Ok(())
    }
}

pub struct JsonStateStore {
    path: PathBuf,
}

impl JsonStateStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

#[async_trait]
impl StateStore for JsonStateStore {
    async fn load(&self) -> Result<BotState, GoldenPayError> {
        if !self.path.exists() {
            return Ok(BotState::default());
        }

        let raw = fs::read_to_string(&self.path).await?;
        Ok(serde_json::from_str(&raw)?)
    }

    async fn save(&self, state: &BotState) -> Result<(), GoldenPayError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let raw = serde_json::to_string_pretty(state)?;
        fs::write(&self.path, raw).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("goldenpay-{name}-{stamp}.json"))
    }

    #[tokio::test]
    async fn json_store_roundtrip() {
        let path = temp_path("state");
        let store = JsonStateStore::new(&path);

        let mut state = BotState::default();
        state.seen_orders.push("ORDER123".to_string());
        state.seen_messages.insert("users-1-2".to_string(), 42);

        store.save(&state).await.unwrap();
        let loaded = store.load().await.unwrap();

        assert_eq!(loaded.seen_orders, vec!["ORDER123".to_string()]);
        assert_eq!(loaded.seen_messages.get("users-1-2"), Some(&42));

        let _ = fs::remove_file(path).await;
    }
}
