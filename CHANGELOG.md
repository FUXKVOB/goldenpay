# Changelog

All notable changes to this project will be documented in this file.

## [1.0.0] - 2026-07-02
### Added
* **Security & Webhook Module**: Added `SecureString`, webhook server, key validation, session health checks, and key rotation (`rotate_key`) (by you).
* **SQLite Storage**: Implemented `SqliteStateStore` for robust state persistence using `rusqlite`.
* **CI/CD Pipeline**: Configured GitHub Actions to automatically run `cargo check`, `cargo fmt`, `clippy`, and `test`.
* **Auto-Pricing (Undercut)**: Added `undercut_price` method to automatically outbid competitors without going below a minimum threshold.
* **Auto-Raise**: Added `raise_offers` and interval-based scheduling to keep offers at the top of the category.
* **Sleep Scheduling**: Introduced `BotOptions::sleep_schedule` to automatically pause bots during nighttime.
* **Proxy Support**: Added proxy usage and connection verification (`validate_proxy`).
* **Chat Interactions**: Added `upload_chat_file`, `send_message`, and `reply_to_review` features.
* **Withdrawals**: Added the ability to automatically request a balance withdrawal (`withdraw()`).
* **New Listing Automation**: Added `create_offer` and `create_offer_with` to create empty or new offers on the fly.
* **Bulk Operations**: Added `deactivate_all_offers` and `delete_all_offers` for sweeping changes across categories.
* **Balance & Stats**: Implemented `fetch_balance` to parse current account balance and `calculate_statistics` to quickly gather revenue and sales statistics.
* **Store Analytics**: Fetch order volume, average check, and unique buyer numbers based on your order history.

### Changed
* Refactored API and documentation for stability and ease of use (by you).
* Updated `Cargo.toml` logic from `include` to `exclude` for cleaner packaging (by you).
* Fixed stochastic test compilation errors (RngExt vs replace) (by you).
* Improved `BotOptions` builder to be more ergonomic and robust.

### Removed
* Internal stubs and outdated experimental paths.

## [0.5.0] - Previous
* Added batch operations, filters, and cleaned up legacy `fetch_all_orders` stub.
