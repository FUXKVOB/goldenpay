use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Authenticated seller account metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    /// FunPay user id.
    pub id: i64,
    /// Visible seller username.
    pub username: String,
    /// CSRF token extracted from page app data.
    pub csrf_token: String,
    /// PHP session id when available.
    pub phpsessid: Option<String>,
}

/// Parsed chat message from a FunPay dialog.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Message id inside the chat stream.
    pub id: i64,
    /// Chat node id like `users-1-2`.
    pub chat_id: String,
    /// Author FunPay user id.
    pub author_id: i64,
    /// Extracted plain text body.
    pub text: Option<String>,
}

/// Lightweight order view returned from the trade orders list.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrderInfo {
    /// Order code without leading `#`.
    pub id: String,
    /// Buyer username shown in the orders list.
    pub buyer_username: String,
    /// Buyer FunPay user id when parsed successfully.
    pub buyer_id: i64,
    /// Chat node id inferred from seller and buyer ids.
    pub chat_id: String,
    /// Description text from the order row.
    pub description: String,
    /// Parsed subcategory name.
    pub subcategory_name: String,
    /// Parsed item amount, defaults to `1`.
    pub amount: i32,
    /// Current order status.
    pub status: OrderStatus,
}

/// Rich order page model parsed from the full order page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderPage {
    /// Order code.
    pub id: String,
    /// Current order status.
    pub status: OrderStatus,
    /// Parsed amount.
    pub amount: i32,
    /// Parsed total sum.
    pub sum: f64,
    /// Currency suffix extracted from the order page.
    pub currency: String,
    /// Buyer FunPay user id.
    pub buyer_id: i64,
    /// Buyer username.
    pub buyer_username: String,
    /// Related chat id.
    pub chat_id: String,
    /// Short description block if present.
    pub short_description: Option<String>,
    /// Full description block if present.
    pub full_description: Option<String>,
    /// Subcategory name if present.
    pub subcategory_name: Option<String>,
    /// Extracted secret values from paid product sections.
    pub secrets: Vec<String>,
    /// Other visible order params as label-value pairs.
    pub params: Vec<(String, String)>,
    /// Parsed review block when available.
    pub review: Option<Review>,
    /// Full original HTML for custom parsing/debugging.
    pub raw_html: String,
}

/// Parsed order review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    /// Count of highlighted stars.
    pub stars: Option<i32>,
    /// Review text body.
    pub text: Option<String>,
}

/// High-level order status used by the crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderStatus {
    Paid,
    Closed,
    Refunded,
}

/// Seller-owned offer from a trade page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Offer {
    pub id: i64,
    pub node_id: i64,
    pub description: String,
    pub price: f64,
    pub currency: String,
    pub active: bool,
}

/// Public market offer visible in listings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketOffer {
    pub id: i64,
    pub node_id: i64,
    pub description: String,
    pub price: f64,
    pub currency: String,
    pub seller_id: i64,
    pub seller_name: String,
    pub seller_online: bool,
    pub seller_rating: Option<f64>,
    pub seller_reviews: u32,
    pub is_promo: bool,
}

/// Partial offer update payload used by `edit_offer`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfferEdit {
    pub quantity: Option<String>,
    pub quantity2: Option<String>,
    pub method: Option<String>,
    pub offer_type: Option<String>,
    pub server_id: Option<String>,
    pub desc_ru: Option<String>,
    pub desc_en: Option<String>,
    pub payment_msg_ru: Option<String>,
    pub payment_msg_en: Option<String>,
    pub summary_ru: Option<String>,
    pub summary_en: Option<String>,
    pub game: Option<String>,
    pub images: Option<String>,
    pub price: Option<String>,
    pub deactivate_after_sale: Option<bool>,
    pub active: Option<bool>,
    pub location: Option<String>,
    pub deleted: Option<bool>,
}

impl OfferEdit {
    /// Merges another patch over the current value, preferring `other` fields.
    pub fn merge(self, other: OfferEdit) -> Self {
        Self {
            quantity: other.quantity.or(self.quantity),
            quantity2: other.quantity2.or(self.quantity2),
            method: other.method.or(self.method),
            offer_type: other.offer_type.or(self.offer_type),
            server_id: other.server_id.or(self.server_id),
            desc_ru: other.desc_ru.or(self.desc_ru),
            desc_en: other.desc_en.or(self.desc_en),
            payment_msg_ru: other.payment_msg_ru.or(self.payment_msg_ru),
            payment_msg_en: other.payment_msg_en.or(self.payment_msg_en),
            summary_ru: other.summary_ru.or(self.summary_ru),
            summary_en: other.summary_en.or(self.summary_en),
            game: other.game.or(self.game),
            images: other.images.or(self.images),
            price: other.price.or(self.price),
            deactivate_after_sale: other.deactivate_after_sale.or(self.deactivate_after_sale),
            active: other.active.or(self.active),
            location: other.location.or(self.location),
            deleted: other.deleted.or(self.deleted),
        }
    }
}

/// Editable offer details together with dynamic custom fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfferDetails {
    pub offer_id: i64,
    pub node_id: i64,
    pub current: OfferEdit,
    pub custom_fields: Vec<OfferField>,
}

/// Dynamic offer form field from the edit page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfferField {
    pub name: String,
    pub label: String,
    pub field_type: OfferFieldType,
    pub value: String,
    pub options: Vec<OfferFieldOption>,
}

/// Select option for a dynamic offer field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfferFieldOption {
    pub value: String,
    pub label: String,
    pub selected: bool,
}

/// Supported dynamic offer field kinds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OfferFieldType {
    Text,
    Textarea,
    Select,
    Checkbox,
    Hidden,
    Unknown(String),
}

/// Parsed category subcategory item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorySubcategory {
    pub id: i64,
    pub name: String,
    pub offer_count: u32,
    pub subcategory_type: CategorySubcategoryType,
    pub is_active: bool,
}

/// Marketplace subcategory family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CategorySubcategoryType {
    Lots,
    Chips,
}

/// Parsed showcase filter definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryFilter {
    pub id: String,
    pub name: String,
    pub filter_type: CategoryFilterType,
    pub options: Vec<CategoryFilterOption>,
}

/// One option inside a showcase filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryFilterOption {
    pub value: String,
    pub label: String,
}

/// Supported showcase filter kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CategoryFilterType {
    Select,
    RadioBox,
    Range,
    Checkbox,
}

/// Persisted bot polling state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BotState {
    /// Known order ids already observed by the bot.
    pub seen_orders: Vec<String>,
    /// Last seen message id per chat id.
    pub seen_messages: HashMap<String, i64>,
}
