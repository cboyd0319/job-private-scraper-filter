use super::{
    reviewed_export::{protocol_error, ReviewedExportSelection},
    reviewed_export_schema::ExportTable,
};
use serde_json::Value;

pub(super) fn sanitize_export_fields(
    data: &mut Value,
    table: &ExportTable,
    selection: ReviewedExportSelection,
) -> Result<(), sqlx::Error> {
    let object = data
        .as_object_mut()
        .ok_or_else(|| protocol_error("Reviewed export row could not be encoded"))?;
    for column in table
        .url_columns
        .split(',')
        .filter(|value| !value.is_empty())
    {
        let sanitized = match object.get(column) {
            Some(Value::String(url)) => sanitized_url(url),
            Some(Value::Null) => continue,
            _ => return Err(protocol_error("Reviewed export URL field is invalid")),
        };
        object.insert(column.to_string(), sanitized);
    }
    for column in table
        .url_or_text_columns
        .split(',')
        .filter(|value| !value.is_empty())
    {
        match object.get(column) {
            Some(Value::String(url)) if is_http_url(url) => {
                object.insert(column.to_string(), Value::Null);
            }
            Some(Value::String(_) | Value::Null) => {}
            _ => return Err(protocol_error("Reviewed export link field is invalid")),
        }
    }
    for column in table
        .json_url_columns
        .split(',')
        .filter(|value| !value.is_empty())
    {
        let raw = object
            .get(column)
            .and_then(Value::as_str)
            .ok_or_else(|| protocol_error("Reviewed export nested data is invalid"))?;
        let mut nested: Value = serde_json::from_str(raw)
            .map_err(|_| protocol_error("Reviewed export nested data is invalid"))?;
        if !nested.is_object() {
            return Err(protocol_error("Reviewed export nested data is invalid"));
        }
        sanitize_nested_json(&mut nested, selection.include_protected_records)?;
        object.insert(
            column.to_string(),
            Value::String(
                serde_json::to_string(&nested)
                    .map_err(|_| protocol_error("Reviewed export nested data is invalid"))?,
            ),
        );
    }
    Ok(())
}

fn sanitize_nested_json(value: &mut Value, include_protected: bool) -> Result<(), sqlx::Error> {
    match value {
        Value::Object(object) => {
            object.retain(|key, _| {
                !is_managed_private_json_key(key)
                    && (include_protected || !is_protected_json_key(key))
            });
            for (key, value) in object {
                if is_url_json_key(key) {
                    match value {
                        Value::String(url) => *value = sanitized_url(url),
                        Value::Null => {}
                        _ => {
                            return Err(protocol_error(
                                "Reviewed export nested URL field is invalid",
                            ));
                        }
                    }
                } else {
                    sanitize_nested_json(value, include_protected)?;
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                sanitize_nested_json(value, include_protected)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn normalized_json_key(key: &str) -> String {
    key.bytes()
        .filter(u8::is_ascii_alphanumeric)
        .map(|byte| byte.to_ascii_lowercase() as char)
        .collect()
}

fn is_url_json_key(key: &str) -> bool {
    let key = normalized_json_key(key);
    matches!(key.as_str(), "url" | "linkedin" | "github" | "website") || key.ends_with("url")
}

fn is_protected_json_key(key: &str) -> bool {
    let key = normalized_json_key(key);
    ["clearance", "military", "veteran", "disability"]
        .iter()
        .any(|marker| key.contains(marker))
}

fn is_managed_private_json_key(key: &str) -> bool {
    let key = normalized_json_key(key);
    [
        "password", "token", "secret", "cookie", "session", "apikey", "webhook",
    ]
    .iter()
    .any(|marker| key.contains(marker))
        || key.ends_with("filepath")
        || matches!(
            key.as_str(),
            "path"
                | "meetinglink"
                | "meetingurl"
                | "challengelink"
                | "challengeurl"
                | "connectionlink"
                | "connectionurl"
        )
}

fn is_http_url(value: &str) -> bool {
    let value = value.trim_start().to_ascii_lowercase();
    value.starts_with("https://") || value.starts_with("http://")
}

fn sanitized_url(value: &str) -> Value {
    let value = value.trim();
    let candidate = if is_http_url(value) {
        value.to_string()
    } else {
        format!("https://{value}")
    };
    jobsentinel_security::canonicalize_user_supplied_job_url(&candidate)
        .map(Value::String)
        .unwrap_or(Value::Null)
}
