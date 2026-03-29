use crate::client::GoldenPaySession;
use crate::error::GoldenPayError;
use crate::models::{OrderInfo, OrderStatus, RunnerResponse};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryItem {
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryItemFormat {
    PlainLines,
    Numbered,
    CodeBlock,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProductInventory {
    pub items: Vec<DeliveryItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryMatch {
    pub product_key: String,
    pub items: Vec<DeliveryItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryResult {
    pub order_id: String,
    pub product_key: String,
    pub delivered: Vec<DeliveryItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessPaidOrderResult {
    pub delivery: DeliveryResult,
    pub message_text: String,
    pub runner_response: RunnerResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryMessageBuilder {
    pub greeting: String,
    pub intro: String,
    pub item_format: DeliveryItemFormat,
    pub include_order_id: bool,
    pub include_product_key: bool,
    pub footer: Option<String>,
}

impl Default for DeliveryMessageBuilder {
    fn default() -> Self {
        Self {
            greeting: "Thanks for your purchase!".to_string(),
            intro: "Your item:".to_string(),
            item_format: DeliveryItemFormat::Numbered,
            include_order_id: true,
            include_product_key: true,
            footer: Some("If you have any questions, reply in this chat.".to_string()),
        }
    }
}

impl DeliveryMessageBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn greeting(mut self, value: impl Into<String>) -> Self {
        self.greeting = value.into();
        self
    }

    pub fn intro(mut self, value: impl Into<String>) -> Self {
        self.intro = value.into();
        self
    }

    pub fn item_format(mut self, value: DeliveryItemFormat) -> Self {
        self.item_format = value;
        self
    }

    pub fn include_order_id(mut self, value: bool) -> Self {
        self.include_order_id = value;
        self
    }

    pub fn include_product_key(mut self, value: bool) -> Self {
        self.include_product_key = value;
        self
    }

    pub fn footer(mut self, value: impl Into<String>) -> Self {
        self.footer = Some(value.into());
        self
    }

    pub fn no_footer(mut self) -> Self {
        self.footer = None;
        self
    }

    pub fn format_items(&self, items: &[DeliveryItem]) -> String {
        match self.item_format {
            DeliveryItemFormat::PlainLines => items
                .iter()
                .map(|item| item.value.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
            DeliveryItemFormat::Numbered => items
                .iter()
                .enumerate()
                .map(|(index, item)| format!("{}. {}", index + 1, item.value))
                .collect::<Vec<_>>()
                .join("\n"),
            DeliveryItemFormat::CodeBlock => format!(
                "```\n{}\n```",
                items
                    .iter()
                    .map(|item| item.value.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
        }
    }

    pub fn build_message(&self, order: &OrderInfo, result: &DeliveryResult) -> String {
        let mut lines = vec![self.greeting.clone()];

        if self.include_order_id {
            lines.push(format!("Order: #{}", result.order_id));
        }

        if self.include_product_key {
            lines.push(format!("Product: {}", result.product_key));
        }

        lines.push(format!("Buyer: {}", order.buyer_username));
        lines.push(self.intro.clone());
        lines.push(self.format_items(&result.delivered));

        if let Some(footer) = &self.footer {
            lines.push(footer.clone());
        }

        lines.join("\n")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliveryError {
    ProductNotFound,
    NotEnoughItems { requested: usize, available: usize },
    AlreadyDelivered,
    OrderNotPaid { status: OrderStatus },
}

impl From<DeliveryError> for GoldenPayError {
    fn from(value: DeliveryError) -> Self {
        GoldenPayError::state(format!("delivery error: {value:?}"))
    }
}

pub trait ProductMatcher: Send + Sync {
    fn matches(&self, product_key: &str, order: &OrderInfo) -> bool;
}

#[async_trait]
pub trait DeliveryMessenger: Send + Sync {
    async fn send_delivery_message(
        &self,
        chat_id: &str,
        text: &str,
    ) -> Result<RunnerResponse, GoldenPayError>;
}

#[async_trait]
impl DeliveryMessenger for GoldenPaySession {
    async fn send_delivery_message(
        &self,
        chat_id: &str,
        text: &str,
    ) -> Result<RunnerResponse, GoldenPayError> {
        self.send_message(chat_id, text).await
    }
}

pub struct ExactSubcategoryMatcher;

impl ProductMatcher for ExactSubcategoryMatcher {
    fn matches(&self, product_key: &str, order: &OrderInfo) -> bool {
        product_key == order.subcategory_name
    }
}

#[derive(Debug, Clone, Default)]
pub struct DeliveryService {
    pub products: HashMap<String, ProductInventory>,
}

impl DeliveryService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_product(
        &mut self,
        product_key: impl Into<String>,
        items: impl IntoIterator<Item = DeliveryItem>,
    ) {
        self.products.insert(
            product_key.into(),
            ProductInventory {
                items: items.into_iter().collect(),
            },
        );
    }

    pub fn match_order<M: ProductMatcher>(
        &self,
        matcher: &M,
        order: &OrderInfo,
    ) -> Result<DeliveryMatch, DeliveryError> {
        let Some((product_key, inventory)) = self
            .products
            .iter()
            .find(|(key, _)| matcher.matches(key, order))
        else {
            return Err(DeliveryError::ProductNotFound);
        };

        let requested = order.amount.max(0) as usize;
        let available = inventory.items.len();
        if available < requested {
            return Err(DeliveryError::NotEnoughItems {
                requested,
                available,
            });
        }

        Ok(DeliveryMatch {
            product_key: product_key.clone(),
            items: inventory.items.iter().take(requested).cloned().collect(),
        })
    }

    pub fn deliver<M: ProductMatcher>(
        &mut self,
        matcher: &M,
        order: &OrderInfo,
    ) -> Result<DeliveryResult, DeliveryError> {
        let matched = self.match_order(matcher, order)?;
        let inventory = self
            .products
            .get_mut(&matched.product_key)
            .expect("inventory must exist after successful match");
        let delivered = inventory
            .items
            .drain(0..matched.items.len())
            .collect::<Vec<_>>();

        Ok(DeliveryResult {
            order_id: order.id.clone(),
            product_key: matched.product_key,
            delivered,
        })
    }

    pub async fn deliver_order<M: ProductMatcher, S: DeliveryStore>(
        &mut self,
        matcher: &M,
        store: &S,
        order: &OrderInfo,
    ) -> Result<DeliveryResult, DeliveryError> {
        if store.is_delivered(&order.id).await {
            return Err(DeliveryError::AlreadyDelivered);
        }

        let result = self.deliver(matcher, order)?;
        store
            .mark_delivered(&result)
            .await
            .map_err(|_| DeliveryError::AlreadyDelivered)?;
        Ok(result)
    }

    pub async fn process_paid_order<M, S, T>(
        &mut self,
        matcher: &M,
        store: &S,
        messenger: &T,
        builder: &DeliveryMessageBuilder,
        order: &OrderInfo,
    ) -> Result<ProcessPaidOrderResult, GoldenPayError>
    where
        M: ProductMatcher,
        S: DeliveryStore,
        T: DeliveryMessenger,
    {
        if order.status != OrderStatus::Paid {
            return Err(DeliveryError::OrderNotPaid {
                status: order.status,
            }
            .into());
        }

        let delivery = self.deliver_order(matcher, store, order).await?;
        let message_text = builder.build_message(order, &delivery);
        let runner_response = messenger
            .send_delivery_message(&order.chat_id, &message_text)
            .await?;

        Ok(ProcessPaidOrderResult {
            delivery,
            message_text,
            runner_response,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveredOrderRecord {
    pub order_id: String,
    pub product_key: String,
    pub delivered: Vec<DeliveryItem>,
}

#[async_trait]
pub trait DeliveryStore: Send + Sync {
    async fn is_delivered(&self, order_id: &str) -> bool;
    async fn mark_delivered(&self, result: &DeliveryResult) -> Result<(), GoldenPayError>;
}

#[derive(Default)]
pub struct MemoryDeliveryStore {
    inner: Arc<Mutex<HashMap<String, DeliveredOrderRecord>>>,
}

impl MemoryDeliveryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl DeliveryStore for MemoryDeliveryStore {
    async fn is_delivered(&self, order_id: &str) -> bool {
        self.inner.lock().await.contains_key(order_id)
    }

    async fn mark_delivered(&self, result: &DeliveryResult) -> Result<(), GoldenPayError> {
        self.inner.lock().await.insert(
            result.order_id.clone(),
            DeliveredOrderRecord {
                order_id: result.order_id.clone(),
                product_key: result.product_key.clone(),
                delivered: result.delivered.clone(),
            },
        );
        Ok(())
    }
}

pub struct JsonDeliveryStore {
    path: PathBuf,
}

impl JsonDeliveryStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    async fn load_all(&self) -> Result<HashMap<String, DeliveredOrderRecord>, GoldenPayError> {
        if !self.path.exists() {
            return Ok(HashMap::new());
        }

        let raw = fs::read_to_string(&self.path).await?;
        Ok(serde_json::from_str(&raw)?)
    }

    async fn save_all(
        &self,
        records: &HashMap<String, DeliveredOrderRecord>,
    ) -> Result<(), GoldenPayError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let raw = serde_json::to_string_pretty(records)?;
        fs::write(&self.path, raw).await?;
        Ok(())
    }
}

#[async_trait]
impl DeliveryStore for JsonDeliveryStore {
    async fn is_delivered(&self, order_id: &str) -> bool {
        self.load_all()
            .await
            .map(|records| records.contains_key(order_id))
            .unwrap_or(false)
    }

    async fn mark_delivered(&self, result: &DeliveryResult) -> Result<(), GoldenPayError> {
        let mut records = self.load_all().await?;
        records.insert(
            result.order_id.clone(),
            DeliveredOrderRecord {
                order_id: result.order_id.clone(),
                product_key: result.product_key.clone(),
                delivered: result.delivered.clone(),
            },
        );
        self.save_all(&records).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::OrderStatus;
    use crate::models::{RunnerObject, RunnerUnknownObject};
    use std::time::{SystemTime, UNIX_EPOCH};
    use tokio::sync::Mutex as TokioMutex;

    fn sample_order() -> OrderInfo {
        OrderInfo {
            id: "ORDER1".to_string(),
            buyer_username: "buyer".to_string(),
            buyer_id: 2,
            chat_id: "users-1-2".to_string(),
            description: "Steam".to_string(),
            subcategory_name: "Steam Keys".to_string(),
            amount: 2,
            status: OrderStatus::Paid,
        }
    }

    #[derive(Default)]
    struct TestMessenger {
        sent: Arc<TokioMutex<Vec<(String, String)>>>,
    }

    #[async_trait]
    impl DeliveryMessenger for TestMessenger {
        async fn send_delivery_message(
            &self,
            chat_id: &str,
            text: &str,
        ) -> Result<RunnerResponse, GoldenPayError> {
            self.sent
                .lock()
                .await
                .push((chat_id.to_string(), text.to_string()));
            Ok(RunnerResponse {
                success: true,
                objects: vec![RunnerObject::Unknown(RunnerUnknownObject {
                    object_type: Some("test".to_string()),
                    id: None,
                    tag: None,
                    raw: serde_json::json!({ "ok": true }),
                })],
                raw: serde_json::json!({ "ok": true }),
            })
        }
    }

    #[test]
    fn delivers_items_from_inventory() {
        let mut service = DeliveryService::new();
        service.add_product(
            "Steam Keys",
            [
                DeliveryItem {
                    value: "KEY-1".to_string(),
                },
                DeliveryItem {
                    value: "KEY-2".to_string(),
                },
                DeliveryItem {
                    value: "KEY-3".to_string(),
                },
            ],
        );

        let result = service
            .deliver(&ExactSubcategoryMatcher, &sample_order())
            .unwrap();
        assert_eq!(result.product_key, "Steam Keys");
        assert_eq!(result.delivered.len(), 2);
        assert_eq!(
            service.products["Steam Keys"].items,
            vec![DeliveryItem {
                value: "KEY-3".to_string()
            }]
        );
    }

    #[tokio::test]
    async fn delivery_store_blocks_duplicate_orders() {
        let mut service = DeliveryService::new();
        service.add_product(
            "Steam Keys",
            [
                DeliveryItem {
                    value: "KEY-1".to_string(),
                },
                DeliveryItem {
                    value: "KEY-2".to_string(),
                },
            ],
        );

        let store = MemoryDeliveryStore::new();
        let first = service
            .deliver_order(&ExactSubcategoryMatcher, &store, &sample_order())
            .await
            .unwrap();
        assert_eq!(first.delivered.len(), 2);

        let second = service
            .deliver_order(&ExactSubcategoryMatcher, &store, &sample_order())
            .await;
        assert!(matches!(second, Err(DeliveryError::AlreadyDelivered)));
    }

    #[tokio::test]
    async fn json_delivery_store_roundtrip() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("goldenpay-delivery-{stamp}.json"));
        let store = JsonDeliveryStore::new(&path);

        let result = DeliveryResult {
            order_id: "ORDERJSON".to_string(),
            product_key: "Steam Keys".to_string(),
            delivered: vec![DeliveryItem {
                value: "KEY-JSON".to_string(),
            }],
        };

        store.mark_delivered(&result).await.unwrap();
        assert!(store.is_delivered("ORDERJSON").await);

        let _ = fs::remove_file(path).await;
    }

    #[test]
    fn builder_formats_numbered_delivery_message() {
        let order = sample_order();
        let result = DeliveryResult {
            order_id: order.id.clone(),
            product_key: "Steam Keys".to_string(),
            delivered: vec![
                DeliveryItem {
                    value: "KEY-1".to_string(),
                },
                DeliveryItem {
                    value: "KEY-2".to_string(),
                },
            ],
        };

        let text = DeliveryMessageBuilder::new().build_message(&order, &result);
        assert!(text.contains("Order: #ORDER1"));
        assert!(text.contains("Product: Steam Keys"));
        assert!(text.contains("1. KEY-1"));
        assert!(text.contains("2. KEY-2"));
    }

    #[tokio::test]
    async fn process_paid_order_sends_message() {
        let order = sample_order();
        let mut service = DeliveryService::new();
        let store = MemoryDeliveryStore::new();
        let messenger = TestMessenger::default();

        service.add_product(
            "Steam Keys",
            [
                DeliveryItem {
                    value: "KEY-1".to_string(),
                },
                DeliveryItem {
                    value: "KEY-2".to_string(),
                },
            ],
        );

        let processed = service
            .process_paid_order(
                &ExactSubcategoryMatcher,
                &store,
                &messenger,
                &DeliveryMessageBuilder::new(),
                &order,
            )
            .await
            .unwrap();

        assert_eq!(processed.delivery.order_id, "ORDER1");
        assert!(processed.message_text.contains("KEY-1"));
        assert_eq!(messenger.sent.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn process_paid_order_rejects_unpaid_status() {
        let mut order = sample_order();
        order.status = OrderStatus::Closed;

        let mut service = DeliveryService::new();
        let store = MemoryDeliveryStore::new();
        let messenger = TestMessenger::default();

        service.add_product(
            "Steam Keys",
            [DeliveryItem {
                value: "KEY-1".to_string(),
            }],
        );

        let error = service
            .process_paid_order(
                &ExactSubcategoryMatcher,
                &store,
                &messenger,
                &DeliveryMessageBuilder::new(),
                &order,
            )
            .await
            .unwrap_err();

        assert!(matches!(error, GoldenPayError::State { .. }));
    }
}
