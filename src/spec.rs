//! `OpenAPI` 3.0+ specification parsing
//!
//! This module provides types and functions for parsing `OpenAPI` specifications
//! and extracting endpoint/schema information for FDW table generation.

use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Raw schema for deserialization — handles OpenAPI 3.1 type arrays.
///
/// OpenAPI 3.1 changed `type` from a string to potentially an array:
/// - 3.0: `"type": "string"` with `"nullable": true`
/// - 3.1: `"type": ["string", "null"]`
///
/// This intermediate struct captures the raw `type` field, then `From<RawSchema>`
/// extracts the actual type and sets `nullable` accordingly.
#[derive(Debug, Deserialize)]
struct RawSchema {
    #[serde(rename = "type")]
    #[serde(default)]
    schema_type: Option<JsonValue>,
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    properties: HashMap<String, Schema>,
    #[serde(default)]
    items: Option<Box<Schema>>,
    #[serde(rename = "$ref")]
    #[serde(default)]
    reference: Option<String>,
    #[serde(default)]
    required: Vec<String>,
    #[serde(default)]
    nullable: bool,
    #[serde(rename = "writeOnly")]
    #[serde(default)]
    write_only: bool,
    #[serde(rename = "allOf")]
    #[serde(default)]
    all_of: Vec<Schema>,
    #[serde(rename = "oneOf")]
    #[serde(default)]
    one_of: Vec<Schema>,
    #[serde(rename = "anyOf")]
    #[serde(default)]
    any_of: Vec<Schema>,
}

impl From<RawSchema> for Schema {
    fn from(raw: RawSchema) -> Self {
        let (schema_type, type_has_null) = match raw.schema_type {
            None => (None, false),
            Some(JsonValue::String(s)) => (Some(s), false),
            Some(JsonValue::Array(arr)) => {
                let has_null = arr.iter().any(|v| v.as_str() == Some("null"));
                let non_null_types: Vec<&str> = arr
                    .iter()
                    .filter_map(|v| v.as_str())
                    .filter(|s| *s != "null")
                    .collect();
                // Multiple non-null types (e.g., ["string", "integer"]) → None (maps to jsonb)
                let actual = if non_null_types.len() == 1 {
                    Some(non_null_types[0].to_string())
                } else {
                    None
                };
                (actual, has_null)
            }
            Some(_) => (None, false),
        };

        Schema {
            schema_type,
            format: raw.format,
            properties: raw.properties,
            items: raw.items,
            reference: raw.reference,
            required: raw.required,
            // nullable if explicitly set OR if type array contains "null"
            nullable: raw.nullable || type_has_null,
            write_only: raw.write_only,
            all_of: raw.all_of,
            one_of: raw.one_of,
            any_of: raw.any_of,
        }
    }
}

/// Represents an `OpenAPI` 3.0+ specification
#[derive(Debug, Deserialize)]
pub struct OpenApiSpec {
    /// OpenAPI version (must be 3.x)
    pub openapi: String,
    #[allow(dead_code)] // Required by OpenAPI spec format
    info: Info,
    #[serde(default)]
    pub servers: Vec<Server>,
    #[serde(default)]
    pub paths: HashMap<String, PathItem>,
    #[serde(default)]
    pub components: Option<Components>,
}

/// API metadata
#[derive(Debug, Deserialize)]
struct Info {
    #[allow(dead_code)]
    title: String,
}

/// Server definition
#[derive(Debug, Deserialize)]
pub struct Server {
    pub url: String,
    #[serde(default)]
    pub variables: HashMap<String, ServerVariable>,
}

/// Server variable with a default value for URL template substitution
#[derive(Debug, Deserialize)]
pub struct ServerVariable {
    pub default: String,
}

/// Path item (GET and POST operations are used for foreign tables)
#[derive(Debug, Deserialize)]
pub struct PathItem {
    #[serde(default)]
    pub get: Option<Operation>,
    #[serde(default)]
    pub post: Option<Operation>,
}

/// Operation definition
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    #[serde(default)]
    pub responses: HashMap<String, Response>,
}

/// Response definition
#[derive(Debug, Deserialize)]
pub struct Response {
    #[serde(rename = "$ref")]
    #[serde(default)]
    pub reference: Option<String>,
    #[serde(default)]
    pub content: HashMap<String, MediaType>,
}

#[derive(Debug, Deserialize)]
pub struct MediaType {
    #[serde(default)]
    pub schema: Option<Schema>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(from = "RawSchema")]
#[allow(clippy::struct_field_names)]
pub struct Schema {
    pub schema_type: Option<String>,
    pub format: Option<String>,
    pub properties: HashMap<String, Self>,
    pub items: Option<Box<Self>>,
    pub reference: Option<String>,
    pub required: Vec<String>,
    pub nullable: bool,
    pub write_only: bool,
    pub all_of: Vec<Self>,
    pub one_of: Vec<Self>,
    pub any_of: Vec<Self>,
}

#[derive(Debug, Deserialize)]
pub struct Components {
    #[serde(default)]
    pub schemas: HashMap<String, Schema>,
    #[serde(default)]
    pub responses: HashMap<String, Response>,
}

impl OpenApiSpec {
    /// Parse an `OpenAPI` spec from a JSON value
    pub fn from_json(json: &JsonValue) -> Result<Self, String> {
        let spec: Self = serde_json::from_value(json.clone())
            .map_err(|e| format!("Failed to parse OpenAPI spec: {e}"))?;

        if !spec.openapi.starts_with("3.") {
            return Err(format!(
                "Unsupported OpenAPI version '{}'. Only OpenAPI 3.x specifications are supported (not Swagger 2.0).",
                spec.openapi
            ));
        }

        Ok(spec)
    }

    /// Parse an `OpenAPI` spec from a JSON string (used in tests)
    #[cfg(test)]
    pub fn from_str(s: &str) -> Result<Self, String> {
        let spec: Self =
            serde_json::from_str(s).map_err(|e| format!("Failed to parse OpenAPI spec: {e}"))?;

        if !spec.openapi.starts_with("3.") {
            return Err(format!(
                "Unsupported OpenAPI version '{}'. Only OpenAPI 3.x specifications are supported (not Swagger 2.0).",
                spec.openapi
            ));
        }

        Ok(spec)
    }

    /// Get the base URL from the spec (first server URL), substituting any variables
    pub fn base_url(&self) -> Option<String> {
        self.servers.first().map(|s| {
            let mut url = s.url.clone();
            for (name, var) in &s.variables {
                url = url.replace(&format!("{{{name}}}"), &var.default);
            }
            url
        })
    }

    /// Get all endpoint paths that support GET or POST operations (for querying).
    ///
    /// Parameterized paths (e.g., `/users/{id}`, `/users/{user_id}/posts`) are
    /// excluded because they require path parameter values from WHERE clauses at
    /// query time. Users should create these tables manually with the appropriate
    /// `endpoint` option containing `{param}` placeholders. See the documentation
    /// for path parameter examples.
    pub fn get_endpoints(&self) -> Vec<EndpointInfo> {
        let mut endpoints = Vec::new();

        for (path, path_item) in &self.paths {
            // Skip parameterized paths — they require WHERE clause values at query time
            // and must be created manually. See docs for path parameter examples.
            if path.contains('{') {
                continue;
            }

            if let Some(ref op) = path_item.get {
                let response_schema = self.get_response_schema(op);
                endpoints.push(EndpointInfo {
                    path: path.clone(),
                    method: "GET".to_string(),
                    response_schema,
                });
            }

            if let Some(ref op) = path_item.post {
                let response_schema = self.get_response_schema(op);
                endpoints.push(EndpointInfo {
                    path: path.clone(),
                    method: "POST".to_string(),
                    response_schema,
                });
            }
        }

        endpoints.sort_by(|a, b| a.path.cmp(&b.path).then(a.method.cmp(&b.method)));
        endpoints
    }

    /// Get the response schema for a successful response (200, 201, 2XX, or default)
    fn get_response_schema(&self, op: &Operation) -> Option<Schema> {
        let response = op
            .responses
            .get("200")
            .or_else(|| op.responses.get("201"))
            .or_else(|| op.responses.get("2XX"))
            .or_else(|| op.responses.get("default"))?;

        // Resolve $ref at the response level (e.g., "$ref": "#/components/responses/Success")
        let resolved_response = response
            .reference
            .as_ref()
            .and_then(|r| self.resolve_response_ref(r))
            .unwrap_or(response);

        let media_type = resolved_response
            .content
            .iter()
            .find(|(k, _)| k.starts_with("application/json"))
            .map(|(_, v)| v)
            .or_else(|| resolved_response.content.values().next())?;

        media_type.schema.clone()
    }

    /// Resolve a $ref to a response in components.responses
    fn resolve_response_ref(&self, reference: &str) -> Option<&Response> {
        let parts: Vec<&str> = reference.trim_start_matches("#/").split('/').collect();
        if parts.len() == 3 && parts[0] == "components" && parts[1] == "responses" {
            self.components.as_ref()?.responses.get(parts[2])
        } else {
            None
        }
    }

    /// Resolve a $ref to its schema
    pub fn resolve_ref(&self, reference: &str) -> Option<&Schema> {
        // Handle refs like "#/components/schemas/User"
        let parts: Vec<&str> = reference.trim_start_matches("#/").split('/').collect();
        if parts.len() == 3 && parts[0] == "components" && parts[1] == "schemas" {
            self.components.as_ref()?.schemas.get(parts[2])
        } else {
            None
        }
    }

    /// Recursively resolve a schema, following $ref pointers and handling composition.
    /// Uses depth limiting to prevent infinite recursion on circular references.
    pub fn resolve_schema(&self, schema: &Schema) -> Schema {
        self.resolve_schema_with_depth(schema, 0)
    }

    /// Maximum depth for schema resolution to prevent stack overflow on circular refs
    const MAX_RESOLVE_DEPTH: usize = 32;

    /// Internal schema resolution with depth tracking
    fn resolve_schema_with_depth(&self, schema: &Schema, depth: usize) -> Schema {
        // Guard against circular references
        if depth > Self::MAX_RESOLVE_DEPTH {
            return schema.clone();
        }

        // First resolve any $ref
        if let Some(ref reference) = schema.reference {
            if let Some(resolved) = self.resolve_ref(reference) {
                let mut result = self.resolve_schema_with_depth(resolved, depth + 1);
                // Merge non-default siblings (OpenAPI 3.1 $ref with siblings)
                if schema.nullable {
                    result.nullable = true;
                }
                if schema.write_only {
                    result.write_only = true;
                }
                for (name, prop) in &schema.properties {
                    result.properties.insert(name.clone(), prop.clone());
                }
                if !schema.required.is_empty() {
                    result.required.extend(schema.required.iter().cloned());
                    result.required.sort();
                    result.required.dedup();
                }
                return result;
            }
        }

        // Handle allOf by merging all properties (intersection - all schemas apply)
        if !schema.all_of.is_empty() {
            return self.merge_schemas_with_depth(&schema.all_of, false, depth + 1);
        }

        // Handle oneOf by merging all possible properties as nullable (union - one of the schemas)
        if !schema.one_of.is_empty() {
            return self.merge_schemas_with_depth(&schema.one_of, true, depth + 1);
        }

        // Handle anyOf by merging all possible properties as nullable (union - any of the schemas)
        if !schema.any_of.is_empty() {
            return self.merge_schemas_with_depth(&schema.any_of, true, depth + 1);
        }

        schema.clone()
    }

    /// Merge multiple schemas into one with depth tracking.
    /// If `make_nullable` is true, all properties become optional (for oneOf/anyOf)
    fn merge_schemas_with_depth(
        &self,
        schemas: &[Schema],
        make_nullable: bool,
        depth: usize,
    ) -> Schema {
        let mut merged = Schema {
            properties: HashMap::new(),
            required: Vec::new(),
            ..Default::default()
        };

        let mut has_any_properties = false;

        for sub_schema in schemas {
            let resolved = self.resolve_schema_with_depth(sub_schema, depth);

            if !resolved.properties.is_empty() {
                has_any_properties = true;
            }

            // Merge properties
            for (name, mut prop_schema) in resolved.properties {
                if make_nullable {
                    prop_schema.nullable = true;
                    // For oneOf/anyOf: keep first definition (most permissive)
                    merged.properties.entry(name).or_insert(prop_schema);
                } else {
                    // For allOf: later schemas refine/override earlier ones
                    // This follows OpenAPI semantics where allOf combines schemas
                    // and later definitions can provide more specific types
                    merged.properties.insert(name, prop_schema);
                }
            }

            // For allOf, all required fields stay required
            // For oneOf/anyOf, nothing is required since we don't know which variant
            if !make_nullable {
                merged.required.extend(resolved.required);
            }
        }

        // Only set type to "object" if at least one sub-schema has properties.
        // Primitive composition (e.g., oneOf: [{type: "string"}, {type: "integer"}])
        // should produce None (→ jsonb), not "object".
        if has_any_properties {
            merged.schema_type = Some("object".to_string());
        }

        // Deduplicate required fields
        merged.required.sort();
        merged.required.dedup();

        merged
    }
}

/// Extracted endpoint information for table generation
#[derive(Debug)]
pub struct EndpointInfo {
    pub path: String,
    pub method: String,
    pub response_schema: Option<Schema>,
}

impl EndpointInfo {
    /// Generate a table name from the endpoint path.
    ///
    /// Uses the full path to avoid collisions (e.g., `/v1/users` and `/v2/users`
    /// become `v1_users` and `v2_users` instead of both becoming `users`).
    /// POST endpoints get a `_post` suffix to avoid collisions with GET tables.
    pub fn table_name(&self) -> String {
        let cleaned = self.path.trim_matches('/');

        let base = if cleaned.is_empty() {
            "unknown".to_string()
        } else {
            // Join path segments with '_' and convert kebab-case to snake_case
            cleaned.replace(['/', '-'], "_")
        };

        if self.method == "POST" {
            format!("{base}_post")
        } else {
            base
        }
    }
}

#[cfg(test)]
#[path = "spec_tests.rs"]
mod tests;
