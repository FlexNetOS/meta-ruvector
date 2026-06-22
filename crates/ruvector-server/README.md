# Ruvector Server

[![Crates.io](https://img.shields.io/crates/v/ruvector-server.svg)](https://crates.io/crates/ruvector-server)
[![Documentation](https://docs.rs/ruvector-server/badge.svg)](https://docs.rs/ruvector-server)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.77%2B-orange.svg)](https://www.rust-lang.org)

**High-performance REST API server for Ruvector vector databases.**

`ruvector-server` provides an HTTP API built on Axum with CORS support, compression, and request tracing. Exposes Ruvector functionality via RESTful endpoints. Part of the [Ruvector](https://github.com/ruvnet/ruvector) ecosystem.

## Why Ruvector Server?

- **Fast**: Built on Axum and Tokio for high throughput
- **CORS, compression, tracing built-in**
- **RESTful API**: Standard HTTP endpoints for collection and point operations
- **Multi-Collection**: Support multiple vector collections

## Features

### Core Capabilities

- **Point Operations**: Insert, get, and delete points (vectors)
- **Search API**: k-NN search
- **Collection Management**: Create and manage collections
- **Health Checks**: Liveness (`/health`) and readiness (`/ready`) probes

### Built-in Middleware

- **CORS Support**: Enabled via `Config::enable_cors`
- **Compression**: Response compression via `Config::enable_compression`
- **Tracing**: Request tracing with `tower-http`

### Planned / Not Yet Implemented

These are roadmap items and are **not** present in the current code:

- **OpenAPI**: Auto-generated API documentation
- **Rate Limiting**: Request rate limiting
- **Authentication**: API key auth

## Installation

Add `ruvector-server` to your `Cargo.toml`:

```toml
[dependencies]
ruvector-server = "0.1.1"
```

## Quick Start

### Start Server

```rust
use ruvector_server::{RuvectorServer, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure the server
    let config = Config {
        host: "0.0.0.0".to_string(),
        port: 6333,
        enable_cors: true,
        enable_compression: true,
    };

    // Build and start the server (consumes self)
    let server = RuvectorServer::with_config(config);
    server.start().await?;

    // Or just use defaults: RuvectorServer::new().start().await?;

    Ok(())
}
```

### API Endpoints

```bash
# Health check
GET /health

# Collections
POST   /collections              # Create collection
GET    /collections              # List collections
GET    /collections/{name}       # Get collection info
DELETE /collections/{name}       # Delete collection

# Vectors
POST   /collections/{name}/vectors       # Insert vector(s)
GET    /collections/{name}/vectors/{id}  # Get vector
DELETE /collections/{name}/vectors/{id}  # Delete vector

# Search
POST   /collections/{name}/search        # k-NN search
POST   /collections/{name}/search/batch  # Batch search
```

### Example Requests

```bash
# Create collection
curl -X POST http://localhost:6333/collections \
  -H "Content-Type: application/json" \
  -d '{
    "name": "documents",
    "dimensions": 384,
    "distance_metric": "cosine"
  }'

# Insert vector
curl -X POST http://localhost:6333/collections/documents/vectors \
  -H "Content-Type: application/json" \
  -d '{
    "id": "doc-1",
    "vector": [0.1, 0.2, 0.3, ...],
    "metadata": {"title": "Hello World"}
  }'

# Search
curl -X POST http://localhost:6333/collections/documents/search \
  -H "Content-Type: application/json" \
  -d '{
    "vector": [0.1, 0.2, 0.3, ...],
    "k": 10,
    "filter": {"category": "tech"}
  }'
```

## API Overview

### Server Types

```rust
// Server configuration (Default available; host 127.0.0.1, port 6333)
pub struct Config {
    pub host: String,
    pub port: u16,
    pub enable_cors: bool,
    pub enable_compression: bool,
}

// Main server
pub struct RuvectorServer { /* ... */ }

impl RuvectorServer {
    pub fn new() -> Self;                        // default Config
    pub fn with_config(config: Config) -> Self;  // custom Config
    pub async fn start(self) -> Result<()>;      // bind and serve (consumes self)
}
```

Request and response bodies are defined in the `routes` module (e.g. `routes::points`,
`routes::collections`); shared handler state lives in `AppState`. Errors are surfaced
through `ruvector_server::Error` / `Result`.

### HTTP Status Codes

```text
200 - Success
201 - Created
400 - Bad Request
404 - Not Found
500 - Internal Error
```

## Docker Deployment

```dockerfile
FROM rust:1.77 as builder
WORKDIR /app
COPY . .
RUN cargo build --release -p ruvector-server

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/ruvector-server /usr/local/bin/
EXPOSE 6333
CMD ["ruvector-server"]
```

```bash
docker build -t ruvector-server .
docker run -p 6333:6333 ruvector-server
```

## Related Crates

- **[ruvector-core](../ruvector-core/)** - Core vector database engine
- **[ruvector-collections](../ruvector-collections/)** - Collection management
- **[ruvector-cli](../ruvector-cli/)** - Command-line interface

## Documentation

- **[Main README](../../README.md)** - Complete project overview
- **[API Documentation](https://docs.rs/ruvector-server)** - Full API reference
- **[GitHub Repository](https://github.com/ruvnet/ruvector)** - Source code

## License

**MIT License** - see [LICENSE](../../LICENSE) for details.

---

<div align="center">

**Part of [Ruvector](https://github.com/ruvnet/ruvector) - Built by [rUv](https://ruv.io)**

[![Star on GitHub](https://img.shields.io/github/stars/ruvnet/ruvector?style=social)](https://github.com/ruvnet/ruvector)

[Documentation](https://docs.rs/ruvector-server) | [Crates.io](https://crates.io/crates/ruvector-server) | [GitHub](https://github.com/ruvnet/ruvector)

</div>
