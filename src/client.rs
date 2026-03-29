use crate::config::GoldenPayConfig;
use crate::error::GoldenPayError;
use crate::models::{
    CategoryFilter, CategorySubcategory, ChatMessage, MarketOffer, Offer, OfferDetails, OfferEdit,
    OrderInfo, OrderPage, UserInfo,
};
use crate::parser::{
    parse_category_filters, parse_category_subcategories, parse_chat_messages, parse_market_offers,
    parse_my_offers, parse_offer_details, parse_order_page, parse_orders, parse_user,
};
use crate::urls::Urls;
use crate::utils::{random_tag, retry_sleep};
use reqwest::header::{ACCEPT, CONTENT_TYPE, COOKIE, ORIGIN, REFERER, SET_COOKIE, USER_AGENT};
use reqwest::{Client, Response};
use serde_json::{Value, json};

#[derive(Clone)]
pub struct GoldenPay {
    http: Client,
    config: GoldenPayConfig,
    urls: Urls,
}

#[derive(Clone)]
pub struct GoldenPaySession {
    http: Client,
    config: GoldenPayConfig,
    urls: Urls,
    user: UserInfo,
}

impl GoldenPay {
    /// Creates a reusable client from configuration.
    pub fn new(config: GoldenPayConfig) -> Result<Self, GoldenPayError> {
        if config.golden_key.trim().is_empty() {
            return Err(GoldenPayError::MissingGoldenKey);
        }

        let mut builder = Client::builder().cookie_store(false);
        if let Some(proxy) = &config.proxy {
            builder = builder.proxy(reqwest::Proxy::all(proxy)?);
        }

        Ok(Self {
            http: builder.build()?,
            urls: Urls::new(config.base_url.clone()),
            config,
        })
    }

    /// Returns the immutable runtime configuration.
    pub fn config(&self) -> &GoldenPayConfig {
        &self.config
    }

    /// Establishes an authenticated session and fetches seller metadata.
    pub async fn connect(&self) -> Result<GoldenPaySession, GoldenPayError> {
        let response = self
            .request_with_retry(|| {
                self.http
                    .get(self.urls.home())
                    .header(USER_AGENT, &self.config.user_agent)
                    .header(
                        COOKIE,
                        format!("golden_key={}; cookie_prefs=1", self.config.golden_key),
                    )
            })
            .await?;

        let set_cookies = collect_set_cookies(&response);
        let body = response.text().await?;
        let user = parse_user(&body, &set_cookies)?;

        Ok(GoldenPaySession {
            http: self.http.clone(),
            config: self.config.clone(),
            urls: self.urls.clone(),
            user,
        })
    }

    async fn request_with_retry<F>(&self, build: F) -> Result<Response, GoldenPayError>
    where
        F: Fn() -> reqwest::RequestBuilder,
    {
        request_with_retry(&self.config, build).await
    }
}

impl GoldenPaySession {
    /// Returns authenticated user metadata.
    pub fn user(&self) -> &UserInfo {
        &self.user
    }

    pub fn poll_interval(&self) -> std::time::Duration {
        self.config.poll_interval
    }

    pub fn config(&self) -> &GoldenPayConfig {
        &self.config
    }

    /// Sends a chat message to a dialog.
    pub async fn send_message(&self, chat_id: &str, text: &str) -> Result<Value, GoldenPayError> {
        let objects_json = serde_json::to_string(&vec![json!({
            "type": "chat_node",
            "id": chat_id,
            "tag": random_tag(),
            "data": { "node": chat_id, "last_message": -1, "content": "" }
        })])?;

        let request_json = json!({
            "action": "chat_message",
            "data": { "node": chat_id, "last_message": -1, "content": text }
        })
        .to_string();

        let payload = format!(
            "objects={}&request={}&csrf_token={}",
            urlencoding::encode(&objects_json),
            urlencoding::encode(&request_json),
            urlencoding::encode(&self.user.csrf_token)
        );

        self.request_runner(payload).await
    }

    /// Fetches current order shortcuts from the trade page.
    pub async fn fetch_orders(&self) -> Result<Vec<OrderInfo>, GoldenPayError> {
        let response = self
            .request_with_retry(|| {
                self.http
                    .get(self.urls.orders_trade())
                    .header(USER_AGENT, &self.config.user_agent)
                    .header(COOKIE, self.cookie_header())
                    .header(ACCEPT, "*/*")
            })
            .await?;
        let body = response.text().await?;
        parse_orders(&body, self.user.id)
    }

    /// Loads a single order page with parsed metadata and secrets.
    pub async fn fetch_order_page(&self, order_id: &str) -> Result<OrderPage, GoldenPayError> {
        let response = self
            .request_with_retry(|| {
                self.http
                    .get(self.urls.order_page(order_id))
                    .header(USER_AGENT, &self.config.user_agent)
                    .header(COOKIE, self.cookie_header())
                    .header(ACCEPT, "*/*")
            })
            .await?;
        let body = response.text().await?;
        parse_order_page(&body, order_id)
    }

    /// Fetches messages from a chat through the runner endpoint.
    pub async fn fetch_chat_messages(
        &self,
        chat_id: &str,
    ) -> Result<Vec<ChatMessage>, GoldenPayError> {
        let objects_json = serde_json::to_string(&vec![json!({
            "type": "chat_node",
            "id": chat_id,
            "tag": random_tag(),
            "data": { "node": chat_id, "last_message": -1, "content": "" }
        })])?;

        let payload = format!(
            "objects={}&request=false&csrf_token={}",
            urlencoding::encode(&objects_json),
            urlencoding::encode(&self.user.csrf_token)
        );

        let value = self.request_runner(payload).await?;
        Ok(parse_chat_messages(chat_id, &value))
    }

    /// Fetches your offers for a given node.
    pub async fn fetch_my_offers(&self, node_id: i64) -> Result<Vec<Offer>, GoldenPayError> {
        let response = self
            .request_with_retry(|| {
                self.http
                    .get(self.urls.lots_trade(node_id))
                    .header(USER_AGENT, &self.config.user_agent)
                    .header(COOKIE, self.cookie_header())
                    .header(ACCEPT, "*/*")
            })
            .await?;
        Ok(parse_my_offers(&response.text().await?, node_id))
    }

    /// Fetches public market offers for a given node.
    pub async fn fetch_market_offers(
        &self,
        node_id: i64,
    ) -> Result<Vec<MarketOffer>, GoldenPayError> {
        let response = self
            .request_with_retry(|| {
                self.http
                    .get(self.urls.lots_page(node_id))
                    .header(USER_AGENT, &self.config.user_agent)
                    .header(COOKIE, self.cookie_header())
                    .header(ACCEPT, "*/*")
            })
            .await?;
        Ok(parse_market_offers(&response.text().await?, node_id))
    }

    /// Loads editable offer details and dynamic custom fields.
    pub async fn fetch_offer_details(
        &self,
        node_id: i64,
        offer_id: i64,
    ) -> Result<OfferDetails, GoldenPayError> {
        let response = self
            .request_with_retry(|| {
                self.http
                    .get(self.urls.offer_edit(node_id, offer_id))
                    .header(USER_AGENT, &self.config.user_agent)
                    .header(COOKIE, self.cookie_header())
                    .header(ACCEPT, "*/*")
            })
            .await?;
        Ok(parse_offer_details(
            &response.text().await?,
            offer_id,
            node_id,
        ))
    }

    /// Applies an offer edit patch on top of current remote values.
    pub async fn edit_offer(
        &self,
        node_id: i64,
        offer_id: i64,
        patch: OfferEdit,
    ) -> Result<Value, GoldenPayError> {
        let current = self.fetch_offer_details(node_id, offer_id).await?.current;
        let merged = current.merge(patch);
        let payload = build_offer_payload(&self.user.csrf_token, offer_id, node_id, &merged);

        let response = self
            .request_with_retry(|| {
                self.http
                    .post(self.urls.offer_save())
                    .header(USER_AGENT, &self.config.user_agent)
                    .header(COOKIE, self.cookie_header())
                    .header(
                        CONTENT_TYPE,
                        "application/x-www-form-urlencoded; charset=UTF-8",
                    )
                    .header(ACCEPT, "application/json, text/javascript, */*; q=0.01")
                    .header(ORIGIN, self.urls.base())
                    .header(REFERER, self.urls.offer_edit(node_id, offer_id))
                    .header("x-requested-with", "XMLHttpRequest")
                    .body(payload.clone())
            })
            .await?;

        Ok(response.json().await?)
    }

    /// Calculates price information for a node.
    pub async fn calc_price(&self, node_id: i64, price: f64) -> Result<Value, GoldenPayError> {
        let payload = format!("nodeId={node_id}&price={}", price as i64);
        let response = self
            .request_with_retry(|| {
                self.http
                    .post(self.urls.lots_calc())
                    .header(USER_AGENT, &self.config.user_agent)
                    .header(COOKIE, self.cookie_header())
                    .header(
                        CONTENT_TYPE,
                        "application/x-www-form-urlencoded; charset=UTF-8",
                    )
                    .header(ACCEPT, "application/json, text/javascript, */*; q=0.01")
                    .header(ORIGIN, self.urls.base())
                    .header("x-requested-with", "XMLHttpRequest")
                    .body(payload.clone())
            })
            .await?;
        Ok(response.json().await?)
    }

    /// Lists subcategories for a given node.
    pub async fn fetch_category_subcategories(
        &self,
        node_id: i64,
    ) -> Result<Vec<CategorySubcategory>, GoldenPayError> {
        let response = self
            .request_with_retry(|| {
                self.http
                    .get(self.urls.lots_page(node_id))
                    .header(USER_AGENT, &self.config.user_agent)
                    .header(COOKIE, self.cookie_header())
                    .header(ACCEPT, "*/*")
            })
            .await?;
        Ok(parse_category_subcategories(&response.text().await?))
    }

    /// Lists available category filters for a given node.
    pub async fn fetch_category_filters(
        &self,
        node_id: i64,
    ) -> Result<Vec<CategoryFilter>, GoldenPayError> {
        let response = self
            .request_with_retry(|| {
                self.http
                    .get(self.urls.lots_page(node_id))
                    .header(USER_AGENT, &self.config.user_agent)
                    .header(COOKIE, self.cookie_header())
                    .header(ACCEPT, "*/*")
            })
            .await?;
        Ok(parse_category_filters(&response.text().await?))
    }

    async fn request_runner(&self, payload: String) -> Result<Value, GoldenPayError> {
        let response = self
            .request_with_retry(|| {
                self.http
                    .post(self.urls.runner())
                    .header(USER_AGENT, &self.config.user_agent)
                    .header(COOKIE, self.cookie_header())
                    .header(
                        CONTENT_TYPE,
                        "application/x-www-form-urlencoded; charset=UTF-8",
                    )
                    .header(ACCEPT, "*/*")
                    .header(ORIGIN, self.urls.base())
                    .header(REFERER, format!("{}/chat/", self.urls.base()))
                    .header("x-requested-with", "XMLHttpRequest")
                    .body(payload.clone())
            })
            .await?;
        Ok(response.json().await?)
    }

    async fn request_with_retry<F>(&self, build: F) -> Result<Response, GoldenPayError>
    where
        F: Fn() -> reqwest::RequestBuilder,
    {
        request_with_retry(&self.config, build).await
    }

    fn cookie_header(&self) -> String {
        match &self.user.phpsessid {
            Some(session) => format!(
                "golden_key={}; cookie_prefs=1; PHPSESSID={session}",
                self.config.golden_key
            ),
            None => format!("golden_key={}; cookie_prefs=1", self.config.golden_key),
        }
    }
}

async fn request_with_retry<F>(
    config: &GoldenPayConfig,
    build: F,
) -> Result<Response, GoldenPayError>
where
    F: Fn() -> reqwest::RequestBuilder,
{
    for attempt in 1..=config.retry.max_attempts {
        match ensure_success(build().send().await).await {
            Ok(response) => return Ok(response),
            Err(error) => {
                let retryable = matches!(
                    error,
                    GoldenPayError::Http { .. }
                        | GoldenPayError::RequestFailed {
                            status: 429 | 500 | 502 | 503 | 504,
                            ..
                        }
                );

                if !retryable || attempt == config.retry.max_attempts {
                    return Err(error);
                }

                retry_sleep(attempt, config.retry.base_delay).await;
            }
        }
    }

    Err(GoldenPayError::parse(
        "request_with_retry",
        "retry loop exited unexpectedly",
    ))
}

async fn ensure_success(
    response: Result<Response, reqwest::Error>,
) -> Result<Response, GoldenPayError> {
    let response = response?;
    let url = response.url().to_string();

    if response.status() == reqwest::StatusCode::FORBIDDEN {
        return Err(GoldenPayError::Unauthorized);
    }

    if response.status().is_success() {
        return Ok(response);
    }

    let status = response.status().as_u16();
    let body = response.text().await.unwrap_or_default();
    Err(GoldenPayError::RequestFailed {
        method: "HTTP",
        url,
        status,
        body,
    })
}

fn collect_set_cookies(response: &Response) -> Vec<String> {
    response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok().map(ToString::to_string))
        .collect()
}

fn build_offer_payload(csrf_token: &str, offer_id: i64, node_id: i64, edit: &OfferEdit) -> String {
    let mut parts = vec![
        format!("csrf_token={}", urlencoding::encode(csrf_token)),
        format!("offer_id={offer_id}"),
        format!("node_id={node_id}"),
        field("location", edit.location.as_deref()),
        field("fields[quantity]", edit.quantity.as_deref()),
        field("fields[quantity2]", edit.quantity2.as_deref()),
        field("fields[method]", edit.method.as_deref()),
        field("fields[type]", edit.offer_type.as_deref()),
        field("server_id", edit.server_id.as_deref()),
        field("fields[desc][ru]", edit.desc_ru.as_deref()),
        field("fields[desc][en]", edit.desc_en.as_deref()),
        field("fields[payment_msg][ru]", edit.payment_msg_ru.as_deref()),
        field("fields[payment_msg][en]", edit.payment_msg_en.as_deref()),
        field("fields[summary][ru]", edit.summary_ru.as_deref()),
        field("fields[summary][en]", edit.summary_en.as_deref()),
        field("fields[game]", edit.game.as_deref()),
        field("fields[images]", edit.images.as_deref()),
        field("price", edit.price.as_deref()),
    ];

    parts.push(if edit.deactivate_after_sale.unwrap_or(false) {
        field("deactivate_after_sale[]", Some("on"))
    } else {
        field("deactivate_after_sale", None)
    });

    parts.push(if edit.active.unwrap_or(true) {
        field("active", Some("on"))
    } else {
        field("active", None)
    });

    parts.push(if edit.deleted.unwrap_or(false) {
        "deleted=1".to_string()
    } else {
        "deleted=".to_string()
    });

    parts.join("&")
}

fn field(key: &str, value: Option<&str>) -> String {
    format!(
        "{}={}",
        urlencoding::encode(key),
        urlencoding::encode(value.unwrap_or_default())
    )
}
