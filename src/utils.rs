use rand::RngExt;
use tokio::time::{Duration, sleep};

pub fn extract_phpsessid(set_cookies: &[String]) -> Option<String> {
    set_cookies.iter().find_map(|cookie| {
        cookie.find("PHPSESSID=").and_then(|offset| {
            let value = &cookie[offset + "PHPSESSID=".len()..];
            let session = value.split(';').next().unwrap_or_default().trim();
            (!session.is_empty()).then(|| session.to_string())
        })
    })
}

pub fn random_tag() -> String {
    let mut rng = rand::rng();
    const ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    (0..10)
        .map(|_| ALPHABET[rng.random_range(0..ALPHABET.len())] as char)
        .collect()
}

pub async fn retry_sleep(attempt: u32, base: Duration) {
    let factor = 2_u32.saturating_pow(attempt.saturating_sub(1));
    sleep(base.saturating_mul(factor)).await;
}
