# Changelog

## 0.2.0

- add typed `PriceCalculation` parsing with extracted seller, buyer, and commission fields
- add typed runner objects for chat nodes and order counters
- add delivery automation layer with inventory matching, message building, and paid-order processing
- add `DeliveryStore` abstractions with memory and JSON implementations
- add `DeliveryMessenger` abstraction for testable high-level delivery flows
- add `process_paid_order` example
- improve public docs for runner and automation APIs
- switch `reqwest` TLS configuration to `native-tls` for more stable Windows builds

## 0.1.1

- add builder-based configuration API
- improve release metadata and documentation
- add parser fixtures and tests
- add publish checklist and examples
- add git repository metadata for the public GitHub repo

## 0.1.0

- initial `goldenpay` release on crates.io
- session-based FunPay client with authenticated session flow
- polling bot with persistent state storage
- proxy and retry configuration
- chat messaging support
- order page parsing
- offer read and edit support
- category and market offer parsing
- examples, fixtures, parser tests, and publish metadata
