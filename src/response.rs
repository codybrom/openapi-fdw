//! Response parsing: data extraction and pagination handling

use serde_json::Value as JsonValue;

use crate::OpenApiFdw;
use crate::bindings::supabase::wrappers::types::FdwError;

impl OpenApiFdw {
    /// Extract the data array from the response, taking ownership to avoid cloning
    pub(crate) fn extract_data(&self, resp: &mut JsonValue) -> Result<Vec<JsonValue>, FdwError> {
        // If response_path is specified, use it
        if let Some(ref path) = self.response_path {
            let data = resp.pointer_mut(path).map(JsonValue::take).ok_or_else(|| {
                let available = resp
                    .as_object()
                    .map(|obj| {
                        obj.keys()
                            .map(|k| format!("'{k}'"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();
                format!(
                    "Response path '{path}' not found in response. \
                         Available top-level keys: [{available}]"
                )
            })?;

            return Self::json_to_rows(data);
        }

        // Direct array response
        if resp.is_array() {
            return Self::json_to_rows(resp.take());
        }

        // Try common wrapper patterns
        if resp.is_object() {
            for key in [
                "data", "results", "items", "records", "entries", "features", "@graph",
            ] {
                if resp.get(key).is_some_and(|d| d.is_array() || d.is_object()) {
                    return Self::json_to_rows(resp[key].take());
                }
            }

            // Single object response
            return Ok(vec![resp.take()]);
        }

        Err(format!(
            "Unable to extract data from response (type: {}). \
             Expected an array, object with a known wrapper key \
             (data, results, items, records, entries, features, @graph), \
             or set response_path in table options.",
            match resp {
                JsonValue::Null => "null",
                JsonValue::Bool(_) => "boolean",
                JsonValue::Number(_) => "number",
                JsonValue::String(_) => "string",
                _ => "unknown",
            }
        ))
    }

    /// Convert a JSON value to a vector of row objects (takes ownership, no cloning)
    pub(crate) fn json_to_rows(data: JsonValue) -> Result<Vec<JsonValue>, FdwError> {
        match data {
            JsonValue::Array(arr) => Ok(arr),
            data if data.is_object() => Ok(vec![data]),
            _ => Err(format!(
                "Response data is not an array or object (got {})",
                match &data {
                    JsonValue::Null => "null",
                    JsonValue::Bool(_) => "boolean",
                    JsonValue::Number(_) => "number",
                    JsonValue::String(_) => "string",
                    _ => "unknown",
                }
            )),
        }
    }

    /// Handle pagination from the response
    pub(crate) fn handle_pagination(&mut self, resp: &JsonValue) {
        self.pagination.clear_next();

        // Try configured cursor path first
        if !self.cursor_path.is_empty() {
            if let Some(value) = Self::extract_non_empty_string(resp, &self.cursor_path) {
                if value.starts_with("http://") || value.starts_with("https://") {
                    self.pagination.next_url = Some(value);
                } else {
                    self.pagination.next_cursor = Some(value);
                }
                return;
            }
        }

        // Only try auto-detection for object responses
        if resp.as_object().is_none() {
            return;
        }

        // Check for next URL in common locations
        let next_url_paths = [
            "/meta/pagination/next",
            "/meta/pagination/next_url",
            "/pagination/next",
            "/pagination/next_url",
            "/links/next",
            "/links/next_url",
            "/next",
            "/next_url",
            "/_links/next/href",
        ];
        for path in &next_url_paths {
            if let Some(url) = Self::extract_non_empty_string(resp, path) {
                self.pagination.next_url = Some(url);
                return;
            }
        }

        // Check for has_more flag with cursor
        let has_more_paths = [
            "/meta/pagination/has_more",
            "/has_more",
            "/pagination/has_more",
        ];
        let has_more = has_more_paths
            .iter()
            .find_map(|p| resp.pointer(p))
            .and_then(JsonValue::as_bool)
            .unwrap_or(false);

        if !has_more {
            return;
        }

        // Find next cursor
        let cursor_paths = [
            "/meta/pagination/next_cursor",
            "/pagination/next_cursor",
            "/next_cursor",
            "/cursor",
        ];
        for path in &cursor_paths {
            if let Some(cursor) = Self::extract_non_empty_string(resp, path) {
                self.pagination.next_cursor = Some(cursor);
                return;
            }
        }
    }

    /// Extract a non-empty string from a JSON pointer path
    pub(crate) fn extract_non_empty_string(json: &JsonValue, path: &str) -> Option<String> {
        json.pointer(path)
            .and_then(JsonValue::as_str)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
    }
}

#[cfg(test)]
#[path = "response_tests.rs"]
mod tests;
