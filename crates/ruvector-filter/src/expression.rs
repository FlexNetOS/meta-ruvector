use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

/// Filter expression for querying vectors by payload
#[derive(Debug, Clone)]
pub enum FilterExpression {
    // Comparison operators
    Eq {
        field: String,
        value: Value,
    },
    Ne {
        field: String,
        value: Value,
    },
    Gt {
        field: String,
        value: Value,
    },
    Gte {
        field: String,
        value: Value,
    },
    Lt {
        field: String,
        value: Value,
    },
    Lte {
        field: String,
        value: Value,
    },

    // Range
    Range {
        field: String,
        gte: Option<Value>,
        lte: Option<Value>,
    },

    // Array operations
    In {
        field: String,
        values: Vec<Value>,
    },

    // Text matching
    Match {
        field: String,
        text: String,
    },

    // Geo operations (basic)
    GeoRadius {
        field: String,
        lat: f64,
        lon: f64,
        radius_m: f64,
    },
    GeoBoundingBox {
        field: String,
        top_left: (f64, f64),
        bottom_right: (f64, f64),
    },

    // Logical operators
    And(Vec<FilterExpression>),
    Or(Vec<FilterExpression>),
    Not(Box<FilterExpression>),

    // Existence check
    Exists {
        field: String,
    },
    IsNull {
        field: String,
    },
}

impl Serialize for FilterExpression {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_json_value().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for FilterExpression {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        Self::from_json_value(value).map_err(de::Error::custom)
    }
}

impl FilterExpression {
    fn to_json_value(&self) -> Value {
        let mut object = Map::new();
        match self {
            Self::Eq { field, value } => {
                object.insert("type".to_string(), Value::String("eq".to_string()));
                object.insert("field".to_string(), Value::String(field.clone()));
                object.insert("value".to_string(), value.clone());
            }
            Self::Ne { field, value } => {
                object.insert("type".to_string(), Value::String("ne".to_string()));
                object.insert("field".to_string(), Value::String(field.clone()));
                object.insert("value".to_string(), value.clone());
            }
            Self::Gt { field, value } => {
                object.insert("type".to_string(), Value::String("gt".to_string()));
                object.insert("field".to_string(), Value::String(field.clone()));
                object.insert("value".to_string(), value.clone());
            }
            Self::Gte { field, value } => {
                object.insert("type".to_string(), Value::String("gte".to_string()));
                object.insert("field".to_string(), Value::String(field.clone()));
                object.insert("value".to_string(), value.clone());
            }
            Self::Lt { field, value } => {
                object.insert("type".to_string(), Value::String("lt".to_string()));
                object.insert("field".to_string(), Value::String(field.clone()));
                object.insert("value".to_string(), value.clone());
            }
            Self::Lte { field, value } => {
                object.insert("type".to_string(), Value::String("lte".to_string()));
                object.insert("field".to_string(), Value::String(field.clone()));
                object.insert("value".to_string(), value.clone());
            }
            Self::Range { field, gte, lte } => {
                object.insert("type".to_string(), Value::String("range".to_string()));
                object.insert("field".to_string(), Value::String(field.clone()));
                object.insert("gte".to_string(), gte.clone().unwrap_or(Value::Null));
                object.insert("lte".to_string(), lte.clone().unwrap_or(Value::Null));
            }
            Self::In { field, values } => {
                object.insert("type".to_string(), Value::String("in".to_string()));
                object.insert("field".to_string(), Value::String(field.clone()));
                object.insert("values".to_string(), Value::Array(values.clone()));
            }
            Self::Match { field, text } => {
                object.insert("type".to_string(), Value::String("match".to_string()));
                object.insert("field".to_string(), Value::String(field.clone()));
                object.insert("text".to_string(), Value::String(text.clone()));
            }
            Self::GeoRadius {
                field,
                lat,
                lon,
                radius_m,
            } => {
                object.insert("type".to_string(), Value::String("geo_radius".to_string()));
                object.insert("field".to_string(), Value::String(field.clone()));
                object.insert("lat".to_string(), Value::from(*lat));
                object.insert("lon".to_string(), Value::from(*lon));
                object.insert("radius_m".to_string(), Value::from(*radius_m));
            }
            Self::GeoBoundingBox {
                field,
                top_left,
                bottom_right,
            } => {
                object.insert(
                    "type".to_string(),
                    Value::String("geo_bounding_box".to_string()),
                );
                object.insert("field".to_string(), Value::String(field.clone()));
                object.insert(
                    "top_left".to_string(),
                    Value::Array(vec![Value::from(top_left.0), Value::from(top_left.1)]),
                );
                object.insert(
                    "bottom_right".to_string(),
                    Value::Array(vec![
                        Value::from(bottom_right.0),
                        Value::from(bottom_right.1),
                    ]),
                );
            }
            Self::And(filters) => {
                object.insert("type".to_string(), Value::String("and".to_string()));
                object.insert(
                    "filters".to_string(),
                    Value::Array(filters.iter().map(Self::to_json_value).collect()),
                );
            }
            Self::Or(filters) => {
                object.insert("type".to_string(), Value::String("or".to_string()));
                object.insert(
                    "filters".to_string(),
                    Value::Array(filters.iter().map(Self::to_json_value).collect()),
                );
            }
            Self::Not(filter) => {
                object.insert("type".to_string(), Value::String("not".to_string()));
                object.insert("filter".to_string(), filter.to_json_value());
            }
            Self::Exists { field } => {
                object.insert("type".to_string(), Value::String("exists".to_string()));
                object.insert("field".to_string(), Value::String(field.clone()));
            }
            Self::IsNull { field } => {
                object.insert("type".to_string(), Value::String("is_null".to_string()));
                object.insert("field".to_string(), Value::String(field.clone()));
            }
        }
        Value::Object(object)
    }

    fn from_json_value(value: Value) -> std::result::Result<Self, String> {
        let mut object = match value {
            Value::Object(object) => object,
            other => return Err(format!("filter expression must be an object, got {other}")),
        };
        let kind = take_string(&mut object, "type")?;
        match kind.as_str() {
            "eq" => Ok(Self::Eq {
                field: take_string(&mut object, "field")?,
                value: take_required(&mut object, "value")?,
            }),
            "ne" => Ok(Self::Ne {
                field: take_string(&mut object, "field")?,
                value: take_required(&mut object, "value")?,
            }),
            "gt" => Ok(Self::Gt {
                field: take_string(&mut object, "field")?,
                value: take_required(&mut object, "value")?,
            }),
            "gte" => Ok(Self::Gte {
                field: take_string(&mut object, "field")?,
                value: take_required(&mut object, "value")?,
            }),
            "lt" => Ok(Self::Lt {
                field: take_string(&mut object, "field")?,
                value: take_required(&mut object, "value")?,
            }),
            "lte" => Ok(Self::Lte {
                field: take_string(&mut object, "field")?,
                value: take_required(&mut object, "value")?,
            }),
            "range" => Ok(Self::Range {
                field: take_string(&mut object, "field")?,
                gte: take_optional(&mut object, "gte"),
                lte: take_optional(&mut object, "lte"),
            }),
            "in" => Ok(Self::In {
                field: take_string(&mut object, "field")?,
                values: take_array(&mut object, "values")?,
            }),
            "match" => Ok(Self::Match {
                field: take_string(&mut object, "field")?,
                text: take_string(&mut object, "text")?,
            }),
            "geo_radius" => Ok(Self::GeoRadius {
                field: take_string(&mut object, "field")?,
                lat: take_f64(&mut object, "lat")?,
                lon: take_f64(&mut object, "lon")?,
                radius_m: take_f64(&mut object, "radius_m")?,
            }),
            "geo_bounding_box" => Ok(Self::GeoBoundingBox {
                field: take_string(&mut object, "field")?,
                top_left: take_pair(&mut object, "top_left")?,
                bottom_right: take_pair(&mut object, "bottom_right")?,
            }),
            "and" => Ok(Self::And(take_filter_array(&mut object)?)),
            "or" => Ok(Self::Or(take_filter_array(&mut object)?)),
            "not" => Ok(Self::Not(Box::new(Self::from_json_value(take_required(
                &mut object,
                "filter",
            )?)?))),
            "exists" => Ok(Self::Exists {
                field: take_string(&mut object, "field")?,
            }),
            "is_null" => Ok(Self::IsNull {
                field: take_string(&mut object, "field")?,
            }),
            other => Err(format!("unknown filter expression type `{other}`")),
        }
    }
}

fn take_required(object: &mut Map<String, Value>, key: &str) -> std::result::Result<Value, String> {
    object
        .remove(key)
        .ok_or_else(|| format!("missing `{key}` in filter expression"))
}

fn take_optional(object: &mut Map<String, Value>, key: &str) -> Option<Value> {
    object.remove(key).filter(|value| !value.is_null())
}

fn take_string(object: &mut Map<String, Value>, key: &str) -> std::result::Result<String, String> {
    match take_required(object, key)? {
        Value::String(value) => Ok(value),
        other => Err(format!("`{key}` must be a string, got {other}")),
    }
}

fn take_array(
    object: &mut Map<String, Value>,
    key: &str,
) -> std::result::Result<Vec<Value>, String> {
    match take_required(object, key)? {
        Value::Array(values) => Ok(values),
        other => Err(format!("`{key}` must be an array, got {other}")),
    }
}

fn take_f64(object: &mut Map<String, Value>, key: &str) -> std::result::Result<f64, String> {
    take_required(object, key)?
        .as_f64()
        .ok_or_else(|| format!("`{key}` must be a number"))
}

fn take_pair(
    object: &mut Map<String, Value>,
    key: &str,
) -> std::result::Result<(f64, f64), String> {
    let values = take_array(object, key)?;
    if values.len() != 2 {
        return Err(format!("`{key}` must contain exactly two numbers"));
    }
    let first = values[0]
        .as_f64()
        .ok_or_else(|| format!("`{key}` first value must be a number"))?;
    let second = values[1]
        .as_f64()
        .ok_or_else(|| format!("`{key}` second value must be a number"))?;
    Ok((first, second))
}

fn take_filter_array(
    object: &mut Map<String, Value>,
) -> std::result::Result<Vec<FilterExpression>, String> {
    let values = object
        .remove("filters")
        .or_else(|| object.remove("0"))
        .ok_or_else(|| "missing `filters` in logical filter expression".to_string())?;
    let filters = match values {
        Value::Array(values) => values,
        other => return Err(format!("`filters` must be an array, got {other}")),
    };
    filters
        .into_iter()
        .map(FilterExpression::from_json_value)
        .collect()
}

impl FilterExpression {
    /// Create an equality filter
    pub fn eq(field: impl Into<String>, value: Value) -> Self {
        Self::Eq {
            field: field.into(),
            value,
        }
    }

    /// Create a not-equal filter
    pub fn ne(field: impl Into<String>, value: Value) -> Self {
        Self::Ne {
            field: field.into(),
            value,
        }
    }

    /// Create a greater-than filter
    pub fn gt(field: impl Into<String>, value: Value) -> Self {
        Self::Gt {
            field: field.into(),
            value,
        }
    }

    /// Create a greater-than-or-equal filter
    pub fn gte(field: impl Into<String>, value: Value) -> Self {
        Self::Gte {
            field: field.into(),
            value,
        }
    }

    /// Create a less-than filter
    pub fn lt(field: impl Into<String>, value: Value) -> Self {
        Self::Lt {
            field: field.into(),
            value,
        }
    }

    /// Create a less-than-or-equal filter
    pub fn lte(field: impl Into<String>, value: Value) -> Self {
        Self::Lte {
            field: field.into(),
            value,
        }
    }

    /// Create a range filter
    pub fn range(field: impl Into<String>, gte: Option<Value>, lte: Option<Value>) -> Self {
        Self::Range {
            field: field.into(),
            gte,
            lte,
        }
    }

    /// Create an IN filter
    pub fn in_values(field: impl Into<String>, values: Vec<Value>) -> Self {
        Self::In {
            field: field.into(),
            values,
        }
    }

    /// Create a text match filter
    pub fn match_text(field: impl Into<String>, text: impl Into<String>) -> Self {
        Self::Match {
            field: field.into(),
            text: text.into(),
        }
    }

    /// Create a geo radius filter
    pub fn geo_radius(field: impl Into<String>, lat: f64, lon: f64, radius_m: f64) -> Self {
        Self::GeoRadius {
            field: field.into(),
            lat,
            lon,
            radius_m,
        }
    }

    /// Create a geo bounding box filter
    pub fn geo_bounding_box(
        field: impl Into<String>,
        top_left: (f64, f64),
        bottom_right: (f64, f64),
    ) -> Self {
        Self::GeoBoundingBox {
            field: field.into(),
            top_left,
            bottom_right,
        }
    }

    /// Create an AND filter
    pub fn and(filters: Vec<FilterExpression>) -> Self {
        Self::And(filters)
    }

    /// Create an OR filter
    pub fn or(filters: Vec<FilterExpression>) -> Self {
        Self::Or(filters)
    }

    /// Create a NOT filter
    // Public API constructor mirrors `and`/`or`; not the `std::ops::Not` trait.
    #[allow(clippy::should_implement_trait)]
    pub fn not(filter: FilterExpression) -> Self {
        Self::Not(Box::new(filter))
    }

    /// Create an EXISTS filter
    pub fn exists(field: impl Into<String>) -> Self {
        Self::Exists {
            field: field.into(),
        }
    }

    /// Create an IS NULL filter
    pub fn is_null(field: impl Into<String>) -> Self {
        Self::IsNull {
            field: field.into(),
        }
    }

    /// Get all field names referenced in this expression
    pub fn get_fields(&self) -> Vec<String> {
        let mut fields = Vec::new();
        self.collect_fields(&mut fields);
        fields.sort();
        fields.dedup();
        fields
    }

    fn collect_fields(&self, fields: &mut Vec<String>) {
        match self {
            Self::Eq { field, .. }
            | Self::Ne { field, .. }
            | Self::Gt { field, .. }
            | Self::Gte { field, .. }
            | Self::Lt { field, .. }
            | Self::Lte { field, .. }
            | Self::Range { field, .. }
            | Self::In { field, .. }
            | Self::Match { field, .. }
            | Self::GeoRadius { field, .. }
            | Self::GeoBoundingBox { field, .. }
            | Self::Exists { field }
            | Self::IsNull { field } => {
                fields.push(field.clone());
            }
            Self::And(exprs) | Self::Or(exprs) => {
                for expr in exprs {
                    expr.collect_fields(fields);
                }
            }
            Self::Not(expr) => {
                expr.collect_fields(fields);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_filter_builders() {
        let filter = FilterExpression::eq("status", json!("active"));
        assert!(matches!(filter, FilterExpression::Eq { .. }));

        let filter = FilterExpression::and(vec![
            FilterExpression::eq("status", json!("active")),
            FilterExpression::gte("age", json!(18)),
        ]);
        assert!(matches!(filter, FilterExpression::And(_)));
    }

    #[test]
    fn test_get_fields() {
        let filter = FilterExpression::and(vec![
            FilterExpression::eq("status", json!("active")),
            FilterExpression::or(vec![
                FilterExpression::gte("age", json!(18)),
                FilterExpression::lt("score", json!(100)),
            ]),
        ]);

        let fields = filter.get_fields();
        assert_eq!(fields, vec!["age", "score", "status"]);
    }

    #[test]
    fn test_serialization() {
        let filter = FilterExpression::eq("status", json!("active"));
        let json = serde_json::to_string(&filter).unwrap();
        let deserialized: FilterExpression = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, FilterExpression::Eq { .. }));
    }
}
