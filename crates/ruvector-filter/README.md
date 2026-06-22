# Ruvector Filter

[![Crates.io](https://img.shields.io/crates/v/ruvector-filter.svg)](https://crates.io/crates/ruvector-filter)
[![Documentation](https://docs.rs/ruvector-filter/badge.svg)](https://docs.rs/ruvector-filter)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.77%2B-orange.svg)](https://www.rust-lang.org)

**Advanced metadata filtering for Ruvector vector search.**

`ruvector-filter` provides a filter expression type for combining vector similarity search with payload (metadata) constraints, backed by per-field payload indices for efficient evaluation. Part of the [Ruvector](https://github.com/ruvnet/ruvector) ecosystem.

## Why Ruvector Filter?

- **Rich Expressions**: Composable boolean filter expressions (`FilterExpression`)
- **Indexed Evaluation**: Filters evaluate against typed payload indices (`PayloadIndexManager`)
- **Type-Safe**: Strongly typed builder constructors
- **JSON Compatible**: Values are `serde_json::Value`; expressions serialize to/from JSON

## Features

### Core Capabilities

- **Comparison Operators**: `eq`, `ne`, `gt`, `gte`, `lt`, `lte`
- **Boolean Logic**: `and`, `or`, `not`
- **Range / Set Queries**: `range`, `in_values`
- **Text Matching**: `match_text`
- **Existence Checks**: `exists`, `is_null`
- **Geo Filters**: `geo_radius`, `geo_bounding_box`
- **Index Types**: `Integer`, `Float`, `Keyword`, `Bool`, `Geo`, `Text`

### Planned / Not Yet Implemented

These are **not** present in the current code:

- **Nested JSON field access** in filters
- **Array operations** (`ANY` / `ALL` / `NONE`)
- **Regex / prefix / suffix string matching** (only `match_text` today)
- **Custom filter functions**

## Installation

Add `ruvector-filter` to your `Cargo.toml`:

```toml
[dependencies]
ruvector-filter = "0.1.1"
```

## Quick Start

### Basic Filtering

```rust
use ruvector_filter::{FilterExpression, PayloadIndexManager, FilterEvaluator, IndexType};
use serde_json::json;

// 1. Build a payload index manager and declare indexed fields
let mut manager = PayloadIndexManager::new();
manager.create_index("status", IndexType::Keyword).unwrap();
manager.create_index("age", IndexType::Integer).unwrap();

// 2. Index some payloads (vector_id -> payload JSON)
manager.index_payload("v1", &json!({"status": "active", "age": 25})).unwrap();
manager.index_payload("v2", &json!({"status": "active", "age": 30})).unwrap();

// 3. Build a filter expression
let filter = FilterExpression::and(vec![
    FilterExpression::eq("status", json!("active")),
    FilterExpression::gte("age", json!(25)),
]);

// 4. Evaluate against the indices -> matching vector IDs
let evaluator = FilterEvaluator::new(&manager);
let matches = evaluator.evaluate(&filter).unwrap();   // HashSet<String>
```

### Complex Expressions

```rust
use ruvector_filter::FilterExpression;
use serde_json::json;

let filter = FilterExpression::and(vec![
    FilterExpression::eq("status", json!("active")),
    FilterExpression::or(vec![
        FilterExpression::gt("priority", json!(5)),
        FilterExpression::in_values("tags", vec![json!("urgent"), json!("important")]),
    ]),
    FilterExpression::not(FilterExpression::eq("archived", json!(true))),
]);

// Range query
let range = FilterExpression::range("price", Some(json!(100.0)), Some(json!(500.0)));
```

### Text & Geo Matching

```rust
use ruvector_filter::FilterExpression;

// Simple text token match (requires an IndexType::Text index on the field)
let text = FilterExpression::match_text("description", "machine");

// Geo radius (requires an IndexType::Geo index on the field)
let near = FilterExpression::geo_radius("location", 40.7128, -74.0060, 1000.0);

// Geo bounding box
let bbox = FilterExpression::geo_bounding_box(
    "location",
    (41.0, -75.0),   // top_left (lat, lon)
    (40.0, -73.0),   // bottom_right (lat, lon)
);
```

## API Overview

### Core Types

```rust
// Filter expression (serde tag = "type", snake_case)
pub enum FilterExpression {
    Eq { field: String, value: Value },
    Ne { field: String, value: Value },
    Gt { field: String, value: Value },
    Gte { field: String, value: Value },
    Lt { field: String, value: Value },
    Lte { field: String, value: Value },
    Range { field: String, gte: Option<Value>, lte: Option<Value> },
    In { field: String, values: Vec<Value> },
    Match { field: String, text: String },
    GeoRadius { field: String, lat: f64, lon: f64, radius_m: f64 },
    GeoBoundingBox { field: String, top_left: (f64, f64), bottom_right: (f64, f64) },
    And(Vec<FilterExpression>),
    Or(Vec<FilterExpression>),
    Not(Box<FilterExpression>),
    Exists { field: String },
    IsNull { field: String },
}

// Index types
pub enum IndexType { Integer, Float, Keyword, Bool, Geo, Text }
```

### Builder Constructors

```rust
impl FilterExpression {
    pub fn eq(field: impl Into<String>, value: Value) -> Self;
    pub fn ne(field: impl Into<String>, value: Value) -> Self;
    pub fn gt(field: impl Into<String>, value: Value) -> Self;
    pub fn gte(field: impl Into<String>, value: Value) -> Self;
    pub fn lt(field: impl Into<String>, value: Value) -> Self;
    pub fn lte(field: impl Into<String>, value: Value) -> Self;
    pub fn range(field: impl Into<String>, gte: Option<Value>, lte: Option<Value>) -> Self;
    pub fn in_values(field: impl Into<String>, values: Vec<Value>) -> Self;
    pub fn match_text(field: impl Into<String>, text: impl Into<String>) -> Self;
    pub fn geo_radius(field: impl Into<String>, lat: f64, lon: f64, radius_m: f64) -> Self;
    pub fn geo_bounding_box(field: impl Into<String>, top_left: (f64, f64), bottom_right: (f64, f64)) -> Self;
    pub fn and(filters: Vec<FilterExpression>) -> Self;
    pub fn or(filters: Vec<FilterExpression>) -> Self;
    pub fn not(filter: FilterExpression) -> Self;
    pub fn exists(field: impl Into<String>) -> Self;
    pub fn is_null(field: impl Into<String>) -> Self;
    pub fn get_fields(&self) -> Vec<String>;
}
```

### Indexing & Evaluation

```rust
impl PayloadIndexManager {
    pub fn new() -> Self;
    pub fn create_index(&mut self, field: &str, index_type: IndexType) -> Result<()>;
    pub fn index_payload(&mut self, vector_id: &str, payload: &Value) -> Result<()>;
}

impl<'a> FilterEvaluator<'a> {
    pub fn new(indices: &'a PayloadIndexManager) -> Self;
    pub fn evaluate(&self, filter: &FilterExpression) -> Result<HashSet<String>>;
}
```

`FilterExpression` derives `Serialize` / `Deserialize`, so expressions round-trip
through JSON via `serde_json`.

## Performance Tips

1. **Put most selective filters first** in `and` expressions
2. **Use `in_values` instead of multiple `or`** for equality checks
3. **Index every field you filter on** with `create_index` before evaluating

## Related Crates

- **[ruvector-core](../ruvector-core/)** - Core vector database engine
- **[ruvector-collections](../ruvector-collections/)** - Collection management

## Documentation

- **[Main README](../../README.md)** - Complete project overview
- **[API Documentation](https://docs.rs/ruvector-filter)** - Full API reference
- **[GitHub Repository](https://github.com/ruvnet/ruvector)** - Source code

## License

**MIT License** - see [LICENSE](../../LICENSE) for details.

---

<div align="center">

**Part of [Ruvector](https://github.com/ruvnet/ruvector) - Built by [rUv](https://ruv.io)**

[![Star on GitHub](https://img.shields.io/github/stars/ruvnet/ruvector?style=social)](https://github.com/ruvnet/ruvector)

[Documentation](https://docs.rs/ruvector-filter) | [Crates.io](https://crates.io/crates/ruvector-filter) | [GitHub](https://github.com/ruvnet/ruvector)

</div>
