use serde_json::Value;

#[derive(Debug, Clone)]
pub struct BridgeRequest {
    pub id: Value,
    pub action: String,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeException {
    pub code: String,
    pub fields: Vec<String>,
}

impl BridgeException {
    pub fn new(code: impl Into<String>, fields: &[&str]) -> Self {
        let code = code.into();
        let fields = if fields.is_empty() {
            vec![code.clone()]
        } else {
            fields.iter().map(|value| value.to_string()).collect()
        };
        Self { code, fields }
    }
}

pub fn response(id: &Value, ok: bool, result: Value, error: Option<(&str, &[&str])>) -> Value {
    let error_value = match error {
        Some((code, fields)) => serde_json::json!({
            "code": code,
            "fields": fields.iter().map(|value| value.to_string()).collect::<Vec<String>>(),
        }),
        None => Value::Null,
    };

    serde_json::json!({
        "id": id,
        "ok": ok,
        "result": result,
        "error": error_value,
    })
}

pub fn event(kind: &str, payload: Value) -> Value {
    serde_json::json!({ "type": kind, "payload": payload })
}
