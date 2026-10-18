use serde_json::Value;

pub fn text(source: &Value, name: &str, fallback: &str) -> String {
    source
        .get(name)
        .and_then(Value::as_str)
        .map(|value| value.to_string())
        .unwrap_or_else(|| fallback.to_string())
}

pub fn boolean(source: &Value, name: &str, fallback: bool) -> bool {
    match source.get(name) {
        Some(Value::Bool(value)) => *value,
        Some(Value::String(value)) => value.parse::<bool>().unwrap_or(fallback),
        _ => fallback,
    }
}

pub fn integer(source: &Value, name: &str, fallback: i64) -> i64 {
    match source.get(name) {
        Some(Value::Number(number)) => number.as_i64().unwrap_or(fallback),
        Some(Value::String(value)) => value.parse::<i64>().unwrap_or(fallback),
        _ => fallback,
    }
}

pub fn string_list(source: &Value, name: &str) -> Option<Vec<String>> {
    match source.get(name) {
        Some(Value::Array(items)) => Some(
            items
                .iter()
                .filter_map(|item| item.as_str().map(|value| value.to_string()))
                .collect(),
        ),
        _ => None,
    }
}

pub fn raw_text(source: &Value) -> Option<String> {
    source.as_str().map(|value| value.to_string())
}
