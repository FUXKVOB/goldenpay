# goldenpay

Production-oriented Rust library for FunPay automation.

## Included

- session-based client
- proxy support
- retry policy
- order polling bot
- state storage
- chat messaging
- order page parsing
- offer read and edit
- price calculation
- category and offer listing

## Quick start

```rust
use goldenpay::{GoldenPay, GoldenPayBot, GoldenPayConfig, GoldenPayEvent, RetryPolicy};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = GoldenPay::new(GoldenPayConfig::builder()
        .golden_key("golden_key_here")
        .state_path("data/state.json")
        .poll_interval(Duration::from_secs(2))
        .retry_policy(RetryPolicy::new(4, Duration::from_millis(250)))
        .build())?;

    let session = client.connect().await?;
    let mut bot = GoldenPayBot::new(session);

    bot.load_state().await?;
    bot.bootstrap().await?;

    bot.run(|event, session| async move {
        match event {
            GoldenPayEvent::NewOrder(order) => {
                session
                    .send_message(&order.chat_id, "Thanks for your order")
                    .await?;
            }
            GoldenPayEvent::NewMessage(message) => {
                println!("{:?}", message.text);
            }
        }

        Ok(())
    })
    .await?;

    Ok(())
}
```

## Main API

- `GoldenPay::connect()`
- `GoldenPaySession::send_message()`
- `GoldenPaySession::fetch_orders()`
- `GoldenPaySession::fetch_order_page()`
- `GoldenPaySession::fetch_offer_details()`
- `GoldenPaySession::edit_offer()`
- `GoldenPaySession::calc_price()`
- `GoldenPayBot::load_state()`
- `GoldenPayBot::bootstrap()`
- `GoldenPayBot::poll_once()`
