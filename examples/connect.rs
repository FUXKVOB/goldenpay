use goldenpay::{GoldenPay, GoldenPayConfig};

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
    println!(
        "Logged in as {} ({})",
        session.user().username,
        session.user().id
    );

    Ok(())
}
