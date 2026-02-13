//! Server configuration: headers and authentication setup

use serde_json::{Map as JsonMap, Value as JsonValue};

use crate::bindings::supabase::wrappers::{
    types::{FdwResult, Options},
    utils,
};

/// Immutable server-level configuration.
///
/// Fields are set once in `init()` from server options and remain constant
/// for the FDW lifetime. A few fields (`page_size`, `page_size_param`,
/// `cursor_param`) can be overridden per-table in `begin_scan`.
#[derive(Debug)]
pub(crate) struct ServerConfig {
    pub(crate) base_url: String,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) spec_url: Option<String>,
    pub(crate) api_key_query: Option<(String, String)>,
    pub(crate) include_attrs: bool,
    pub(crate) page_size: usize,
    pub(crate) page_size_param: String,
    pub(crate) cursor_param: String,
    pub(crate) max_pages: usize,
    pub(crate) max_response_bytes: usize,
    pub(crate) debug_timing: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            headers: Vec::new(),
            spec_url: None,
            api_key_query: None,
            include_attrs: false,
            page_size: 0,
            page_size_param: String::new(),
            cursor_param: String::new(),
            max_pages: 1000,
            max_response_bytes: 50 * 1024 * 1024, // 50 MiB
            debug_timing: false,
        }
    }
}

impl ServerConfig {
    /// Configure request headers from server options
    pub(crate) fn configure_headers(&mut self, opts: &Options) -> FdwResult {
        self.headers
            .push(("content-type".to_owned(), "application/json".to_string()));

        // Optional User-Agent header (some APIs require this for identification)
        if let Some(user_agent) = opts.get("user_agent") {
            self.headers.push(("user-agent".to_owned(), user_agent));
        }

        // Optional Accept header for content negotiation (JSON, XML, JSON-LD, GeoJSON etc.)
        if let Some(accept) = opts.get("accept") {
            self.headers.push(("accept".to_owned(), accept));
        }

        // Custom headers as JSON object: '{"Feature-Flags": "value", "X-Custom": "value"}'
        if let Some(headers_json) = opts.get("headers") {
            let headers: JsonMap<String, JsonValue> = serde_json::from_str(&headers_json)
                .map_err(|e| format!("Invalid JSON for 'headers' option: {e}"))?;
            for (key, value) in headers {
                if let Some(v) = value.as_str() {
                    self.headers.push((key.to_lowercase(), v.to_string()));
                } else {
                    return Err(format!(
                        "Invalid non-string value for header '{key}' in 'headers' option"
                    ));
                }
            }
        }

        Ok(())
    }

    /// Configure authentication from server options
    pub(crate) fn configure_auth(&mut self, opts: &Options) -> FdwResult {
        // API Key authentication
        let api_key = opts.get("api_key").or_else(|| {
            opts.get("api_key_id")
                .and_then(|key_id| utils::get_vault_secret(&key_id))
        });

        // Bearer token authentication (alternative to api_key)
        let bearer_token = opts.get("bearer_token").or_else(|| {
            opts.get("bearer_token_id")
                .and_then(|token_id| utils::get_vault_secret(&token_id))
        });

        // Enforce mutual exclusivity — both would emit duplicate auth headers
        if api_key.is_some() && bearer_token.is_some() {
            return Err(
                "Cannot use both api_key/api_key_id and bearer_token/bearer_token_id. \
                 Choose one authentication method."
                    .to_string(),
            );
        }

        if let Some(key) = api_key {
            let location = opts.require_or("api_key_location", "header");

            if location == "query" {
                // API key sent as query parameter (e.g., ?api_key=xxx)
                let param_name = opts.require_or("api_key_header", "api_key");
                self.api_key_query = Some((param_name, key));
            } else if location == "cookie" {
                // API key sent as cookie (e.g., Cookie: session=xxx)
                let cookie_name = opts.require_or("api_key_header", "api_key");
                self.headers
                    .push(("cookie".to_owned(), format!("{cookie_name}={key}")));
            } else {
                // API key sent as header (default)
                let header_name = opts.require_or("api_key_header", "Authorization");
                let prefix = opts.get("api_key_prefix");

                let header_value = match (header_name.as_str(), prefix) {
                    ("Authorization", None) => format!("Bearer {key}"),
                    (_, Some(p)) => format!("{p} {key}"),
                    (_, None) => key,
                };

                self.headers
                    .push((header_name.to_lowercase(), header_value));
            }
        }

        if let Some(token) = bearer_token {
            self.headers
                .push(("authorization".to_owned(), format!("Bearer {token}")));
        }

        Ok(())
    }
}
