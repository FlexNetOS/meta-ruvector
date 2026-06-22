# rvf-server

TCP/HTTP streaming server exposing the RuVector Format runtime via a REST API.

## Overview

`rvf-server` wraps `rvf-runtime` in an Axum-based server for networked vector operations, exposing three entrypoints:

- **REST API** (`http`) -- HTTP endpoints for vector operations over the store
- **TCP streaming** (`tcp`) -- a binary TCP protocol for inter-agent vector exchange
- **WebSocket live events** (`ws`) -- a live event stream broadcasting store activity over the HTTP server

## Usage

```bash
cargo run -p rvf-server -- --port 8080
```

## License

MIT OR Apache-2.0
