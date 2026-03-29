use goldenpay::{
    DeliveryItem, DeliveryMessageBuilder, DeliveryService, ExactSubcategoryMatcher, GoldenPay,
    GoldenPayConfig, JsonDeliveryStore, OrderStatus,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let golden_key = std::env::var("FUNPAY_GOLDEN_KEY")?;

    let client = GoldenPay::new(
        GoldenPayConfig::builder()
            .golden_key(golden_key)
            .state_path("data/goldenpay-state.json")
            .build(),
    )?;

    let session = client.connect().await?;
    let store = JsonDeliveryStore::new("data/deliveries.json");
    let mut delivery = DeliveryService::new();

    delivery.add_product(
        "Steam Keys",
        [
            DeliveryItem {
                value: "KEY-AAA".to_string(),
            },
            DeliveryItem {
                value: "KEY-BBB".to_string(),
            },
        ],
    );

    let Some(order) = session
        .fetch_orders()
        .await?
        .into_iter()
        .find(|order| order.status == OrderStatus::Paid)
    else {
        println!("No paid orders found");
        return Ok(());
    };

    let result = delivery
        .process_paid_order(
            &ExactSubcategoryMatcher,
            &store,
            &session,
            &DeliveryMessageBuilder::new(),
            &order,
        )
        .await?;

    println!(
        "Delivered {} item(s) for order {}",
        result.delivery.delivered.len(),
        result.delivery.order_id
    );

    Ok(())
}
