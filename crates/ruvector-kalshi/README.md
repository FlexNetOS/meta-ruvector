# ruvector-kalshi

Kalshi exchange integration for the RuVector Neural Trader (ADR-153).

## Overview

`ruvector-kalshi` connects the Kalshi prediction-market exchange to the RuVector Neural Trader pipeline. It handles RSA-PSS-SHA256 request signing against Kalshi's REST API, typed DTOs for market/event/order/fill payloads, and normalization of those payloads into `neural_trader_core::MarketEvent` so downstream coherence, attention, and replay stages run unchanged. Live REST calls are gated behind a runtime flag, and secrets load from Google Cloud Secret Manager or a local PEM file.

## Key API

- `KALSHI_VENUE_ID`, `KALSHI_API_URL`, `KALSHI_WS_URL`, `KALSHI_PRICE_FP_SCALE` — venue constants and fixed-point price scaling.
- `KalshiError` / `Result<T>` — crate-wide error type and result alias.
- `auth` — RSA-PSS-SHA256 request signing.
- `models` — typed Kalshi market/event/order/fill DTOs.
- `normalize` — conversion from Kalshi payloads into `neural_trader_core::MarketEvent`.
- `rest`, `rate_limit` — REST client scaffold and rate limiting.
- `ws`, `ws_client` — live WebSocket transport.
- `secrets` — secret loading (GCS Secret Manager / local PEM).
- `strategy_adapter`, `brain` — integration with the neural-trader strategy and brain layers.

## License

MIT OR Apache-2.0
