use super::*;

#[test]
fn test_parse_minimal_spec() {
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test API", "version": "1.0"},
        "paths": {}
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    assert_eq!(spec.openapi, "3.0.0");
    assert_eq!(spec.info.title, "Test API");
}

#[test]
fn test_endpoint_table_name() {
    let endpoint = EndpointInfo {
        path: "/api/v1/user-accounts".to_string(),
        method: "GET".to_string(),
        response_schema: None,
    };
    assert_eq!(endpoint.table_name(), "api_v1_user_accounts");

    // Single segment
    let endpoint = EndpointInfo {
        path: "/users".to_string(),
        method: "GET".to_string(),
        response_schema: None,
    };
    assert_eq!(endpoint.table_name(), "users");

    // Collision avoidance: different versions produce different names
    let v1 = EndpointInfo {
        path: "/v1/users".to_string(),
        method: "GET".to_string(),
        response_schema: None,
    };
    let v2 = EndpointInfo {
        path: "/v2/users".to_string(),
        method: "GET".to_string(),
        response_schema: None,
    };
    assert_ne!(v1.table_name(), v2.table_name());

    // Empty path
    let endpoint = EndpointInfo {
        path: "/".to_string(),
        method: "GET".to_string(),
        response_schema: None,
    };
    assert_eq!(endpoint.table_name(), "unknown");
}

#[test]
fn test_resolve_ref() {
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {},
        "components": {
            "schemas": {
                "User": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"},
                        "name": {"type": "string"}
                    },
                    "required": ["id"]
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let user_schema = spec.resolve_ref("#/components/schemas/User").unwrap();

    assert_eq!(user_schema.schema_type, Some("object".to_string()));
    assert!(user_schema.properties.contains_key("id"));
    assert!(user_schema.properties.contains_key("name"));
    assert!(user_schema.required.contains(&"id".to_string()));
}

#[test]
fn test_allof_merges_properties() {
    let spec_json = r##"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {},
        "components": {
            "schemas": {
                "Base": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"}
                    },
                    "required": ["id"]
                },
                "Extended": {
                    "allOf": [
                        {"$ref": "#/components/schemas/Base"},
                        {
                            "type": "object",
                            "properties": {
                                "name": {"type": "string"},
                                "email": {"type": "string"}
                            },
                            "required": ["name"]
                        }
                    ]
                }
            }
        }
    }"##;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let extended = spec.resolve_ref("#/components/schemas/Extended").unwrap();
    let resolved = spec.resolve_schema(extended);

    // Should have all properties from both schemas
    assert!(resolved.properties.contains_key("id"));
    assert!(resolved.properties.contains_key("name"));
    assert!(resolved.properties.contains_key("email"));

    // Required from both should be merged
    assert!(resolved.required.contains(&"id".to_string()));
    assert!(resolved.required.contains(&"name".to_string()));
}

#[test]
fn test_oneof_merges_as_nullable() {
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {},
        "components": {
            "schemas": {
                "Response": {
                    "oneOf": [
                        {
                            "type": "object",
                            "properties": {
                                "user_id": {"type": "string"},
                                "user_name": {"type": "string"}
                            },
                            "required": ["user_id"]
                        },
                        {
                            "type": "object",
                            "properties": {
                                "org_id": {"type": "string"},
                                "org_name": {"type": "string"}
                            },
                            "required": ["org_id"]
                        }
                    ]
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let response = spec.resolve_ref("#/components/schemas/Response").unwrap();
    let resolved = spec.resolve_schema(response);

    // Should have properties from all variants
    assert!(resolved.properties.contains_key("user_id"));
    assert!(resolved.properties.contains_key("user_name"));
    assert!(resolved.properties.contains_key("org_id"));
    assert!(resolved.properties.contains_key("org_name"));

    // All properties should be nullable (since we don't know which variant)
    assert!(resolved.properties.get("user_id").unwrap().nullable);
    assert!(resolved.properties.get("org_id").unwrap().nullable);

    // Nothing should be required for oneOf
    assert!(resolved.required.is_empty());
}

#[test]
fn test_anyof_merges_as_nullable() {
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {},
        "components": {
            "schemas": {
                "Flexible": {
                    "anyOf": [
                        {
                            "type": "object",
                            "properties": {
                                "name": {"type": "string"}
                            }
                        },
                        {
                            "type": "object",
                            "properties": {
                                "title": {"type": "string"}
                            }
                        }
                    ]
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let flexible = spec.resolve_ref("#/components/schemas/Flexible").unwrap();
    let resolved = spec.resolve_schema(flexible);

    // Should have properties from all variants
    assert!(resolved.properties.contains_key("name"));
    assert!(resolved.properties.contains_key("title"));

    // All should be nullable
    assert!(resolved.properties.get("name").unwrap().nullable);
    assert!(resolved.properties.get("title").unwrap().nullable);
}

#[test]
fn test_nested_ref_resolution() {
    let spec_json = r##"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {},
        "components": {
            "schemas": {
                "Address": {
                    "type": "object",
                    "properties": {
                        "street": {"type": "string"},
                        "city": {"type": "string"}
                    }
                },
                "Person": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "address": {"$ref": "#/components/schemas/Address"}
                    }
                }
            }
        }
    }"##;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let person = spec.resolve_ref("#/components/schemas/Person").unwrap();
    let resolved = spec.resolve_schema(person);

    assert!(resolved.properties.contains_key("name"));
    assert!(resolved.properties.contains_key("address"));

    // The address property should still have a $ref (we resolve at property level when needed)
    let address_prop = resolved.properties.get("address").unwrap();
    assert!(address_prop.reference.is_some() || !address_prop.properties.is_empty());
}

#[test]
fn test_allof_later_schema_overrides_earlier() {
    // Test that for allOf, later schemas override earlier ones
    // This is important for inheritance patterns where a child refines a parent's type
    let spec_json = r##"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {},
        "components": {
            "schemas": {
                "Base": {
                    "type": "object",
                    "properties": {
                        "status": {"type": "string"},
                        "id": {"type": "integer"}
                    }
                },
                "Refined": {
                    "allOf": [
                        {"$ref": "#/components/schemas/Base"},
                        {
                            "type": "object",
                            "properties": {
                                "status": {
                                    "type": "string",
                                    "format": "enum"
                                },
                                "extra": {"type": "boolean"}
                            }
                        }
                    ]
                }
            }
        }
    }"##;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let refined = spec.resolve_ref("#/components/schemas/Refined").unwrap();
    let resolved = spec.resolve_schema(refined);

    // Should have all properties
    assert!(resolved.properties.contains_key("status"));
    assert!(resolved.properties.contains_key("id"));
    assert!(resolved.properties.contains_key("extra"));

    // The 'status' property should be from the later schema (has format: "enum")
    // The base schema's status has no format, so if we get "enum", the later one won
    let status_prop = resolved.properties.get("status").unwrap();
    assert_eq!(
        status_prop.format,
        Some("enum".to_string()),
        "Later allOf schema should override earlier schema's property definition"
    );
}

// --- OpenAPI 3.1 type array tests ---

#[test]
fn test_openapi_31_type_string_null() {
    // OpenAPI 3.1: "type": ["string", "null"] should parse as type=string, nullable=true
    let spec_json = r#"{
        "openapi": "3.1.0",
        "info": {"title": "Test 3.1", "version": "1.0"},
        "paths": {},
        "components": {
            "schemas": {
                "User": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "nickname": {"type": ["string", "null"]},
                        "age": {"type": ["integer", "null"]}
                    },
                    "required": ["name"]
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let user = spec.resolve_ref("#/components/schemas/User").unwrap();

    // name: plain string, not nullable
    let name_prop = user.properties.get("name").unwrap();
    assert_eq!(name_prop.schema_type, Some("string".to_string()));
    assert!(!name_prop.nullable);

    // nickname: ["string", "null"] → string + nullable
    let nickname_prop = user.properties.get("nickname").unwrap();
    assert_eq!(nickname_prop.schema_type, Some("string".to_string()));
    assert!(nickname_prop.nullable);

    // age: ["integer", "null"] → integer + nullable
    let age_prop = user.properties.get("age").unwrap();
    assert_eq!(age_prop.schema_type, Some("integer".to_string()));
    assert!(age_prop.nullable);
}

#[test]
fn test_openapi_31_type_array_without_null() {
    // OpenAPI 3.1: "type": ["string"] (single-element array without null)
    let schema: Schema = serde_json::from_str(r#"{"type": ["string"]}"#).unwrap();
    assert_eq!(schema.schema_type, Some("string".to_string()));
    assert!(!schema.nullable);
}

#[test]
fn test_openapi_30_type_string_still_works() {
    // OpenAPI 3.0: plain string type should still work
    let schema: Schema = serde_json::from_str(r#"{"type": "string"}"#).unwrap();
    assert_eq!(schema.schema_type, Some("string".to_string()));
    assert!(!schema.nullable);
}

#[test]
fn test_openapi_30_nullable_flag_still_works() {
    // OpenAPI 3.0: nullable as a separate flag
    let schema: Schema = serde_json::from_str(r#"{"type": "string", "nullable": true}"#).unwrap();
    assert_eq!(schema.schema_type, Some("string".to_string()));
    assert!(schema.nullable);
}

#[test]
fn test_openapi_31_type_mapping_with_spec() {
    // Verify that type arrays produce correct PostgreSQL type mappings
    let spec_json = r#"{
        "openapi": "3.1.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {
            "/records": {
                "get": {
                    "responses": {
                        "200": {
                            "description": "ok",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "array",
                                        "items": {
                                            "type": "object",
                                            "properties": {
                                                "id": {"type": "integer"},
                                                "name": {"type": ["string", "null"]},
                                                "score": {"type": ["number", "null"], "format": "float"},
                                                "active": {"type": ["boolean", "null"]}
                                            },
                                            "required": ["id"]
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let endpoints = spec.get_endpoints();
    assert_eq!(endpoints.len(), 1);

    let schema = endpoints[0].response_schema.as_ref().unwrap();
    let resolved = spec.resolve_schema(schema);
    let items = resolved.items.as_ref().unwrap();

    // name should be string type, not jsonb (the old bug)
    assert_eq!(
        items.properties.get("name").unwrap().schema_type,
        Some("string".to_string())
    );
    assert_eq!(
        items.properties.get("score").unwrap().schema_type,
        Some("number".to_string())
    );
    assert_eq!(
        items.properties.get("active").unwrap().schema_type,
        Some("boolean".to_string())
    );
}

// --- Circular reference tests ---

#[test]
fn test_circular_ref_depth_limit() {
    // Self-referential schema should not stack overflow
    let spec_json = r##"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {},
        "components": {
            "schemas": {
                "TreeNode": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"},
                        "children": {
                            "type": "array",
                            "items": {"$ref": "#/components/schemas/TreeNode"}
                        }
                    }
                }
            }
        }
    }"##;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let node = spec.resolve_ref("#/components/schemas/TreeNode").unwrap();
    // Should not stack overflow — depth limit kicks in
    let resolved = spec.resolve_schema(node);
    assert!(resolved.properties.contains_key("id"));
    assert!(resolved.properties.contains_key("children"));
}

#[test]
fn test_mutual_circular_refs() {
    // A references B which references A
    let spec_json = r##"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {},
        "components": {
            "schemas": {
                "SchemaA": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "b_ref": {"$ref": "#/components/schemas/SchemaB"}
                    }
                },
                "SchemaB": {
                    "type": "object",
                    "properties": {
                        "value": {"type": "integer"},
                        "a_ref": {"$ref": "#/components/schemas/SchemaA"}
                    }
                }
            }
        }
    }"##;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let schema_a = spec.resolve_ref("#/components/schemas/SchemaA").unwrap();
    let resolved = spec.resolve_schema(schema_a);
    assert!(resolved.properties.contains_key("name"));
    assert!(resolved.properties.contains_key("b_ref"));
}

// --- Deep allOf chain tests (Box-style inheritance) ---

#[test]
fn test_deep_allof_chain() {
    // FileBase → FileMini → File → FileFull (4-level chain)
    let spec_json = r##"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {},
        "components": {
            "schemas": {
                "FileBase": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"},
                        "type": {"type": "string"}
                    },
                    "required": ["id"]
                },
                "FileMini": {
                    "allOf": [
                        {"$ref": "#/components/schemas/FileBase"},
                        {
                            "type": "object",
                            "properties": {
                                "name": {"type": "string"},
                                "size": {"type": "integer"}
                            }
                        }
                    ]
                },
                "File": {
                    "allOf": [
                        {"$ref": "#/components/schemas/FileMini"},
                        {
                            "type": "object",
                            "properties": {
                                "content_type": {"type": "string"},
                                "created_at": {"type": "string", "format": "date-time"}
                            }
                        }
                    ]
                },
                "FileFull": {
                    "allOf": [
                        {"$ref": "#/components/schemas/File"},
                        {
                            "type": "object",
                            "properties": {
                                "permissions": {"type": "object"},
                                "version_number": {"type": "integer"}
                            }
                        }
                    ]
                }
            }
        }
    }"##;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let full = spec.resolve_ref("#/components/schemas/FileFull").unwrap();
    let resolved = spec.resolve_schema(full);

    // Should have all properties from the entire chain
    assert!(
        resolved.properties.contains_key("id"),
        "Missing id from FileBase"
    );
    assert!(
        resolved.properties.contains_key("type"),
        "Missing type from FileBase"
    );
    assert!(
        resolved.properties.contains_key("name"),
        "Missing name from FileMini"
    );
    assert!(
        resolved.properties.contains_key("size"),
        "Missing size from FileMini"
    );
    assert!(
        resolved.properties.contains_key("content_type"),
        "Missing content_type from File"
    );
    assert!(
        resolved.properties.contains_key("created_at"),
        "Missing created_at from File"
    );
    assert!(
        resolved.properties.contains_key("permissions"),
        "Missing permissions from FileFull"
    );
    assert!(
        resolved.properties.contains_key("version_number"),
        "Missing version_number from FileFull"
    );

    // id should be required (from FileBase)
    assert!(resolved.required.contains(&"id".to_string()));
}

// --- Swagger 2.0 rejection ---

#[test]
fn test_swagger_20_rejected() {
    let spec_json = r#"{
        "swagger": "2.0",
        "info": {"title": "Old API", "version": "1.0"},
        "paths": {}
    }"#;

    let result = OpenApiSpec::from_str(spec_json);
    assert!(result.is_err());
}

// --- Parameterized path exclusion ---

#[test]
fn test_parameterized_paths_excluded_from_endpoints() {
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {
            "/items": {
                "get": {
                    "responses": {"200": {"description": "ok"}}
                }
            },
            "/items/{id}": {
                "get": {
                    "responses": {"200": {"description": "ok"}}
                }
            },
            "/users/{user_id}/posts": {
                "get": {
                    "responses": {"200": {"description": "ok"}}
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let endpoints = spec.get_endpoints();

    // Only /items should be included — parameterized paths are excluded
    assert_eq!(endpoints.len(), 1);
    assert_eq!(endpoints[0].path, "/items");
}

// --- Response schema extraction from non-200 codes ---

#[test]
fn test_response_schema_from_201() {
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {
            "/created": {
                "get": {
                    "responses": {
                        "201": {
                            "description": "Created",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "id": {"type": "string"},
                                            "created": {"type": "boolean"}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let endpoints = spec.get_endpoints();
    assert_eq!(endpoints.len(), 1);

    let schema = endpoints[0].response_schema.as_ref().unwrap();
    assert!(schema.properties.contains_key("id"));
    assert!(schema.properties.contains_key("created"));
}

#[test]
fn test_no_schema_endpoint() {
    // Endpoint with no content schema should still be returned with None schema
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {
            "/health": {
                "get": {
                    "responses": {
                        "200": {
                            "description": "Healthy"
                        }
                    }
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let endpoints = spec.get_endpoints();
    assert_eq!(endpoints.len(), 1);
    assert!(endpoints[0].response_schema.is_none());
}

// --- anyOf with null variant (OpenAPI 3.1 pattern) ---

#[test]
fn test_anyof_with_null_variant() {
    // OpenAPI 3.1 uses anyOf: [{type: string}, {type: "null"}] for nullable
    let spec_json = r#"{
        "openapi": "3.1.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {},
        "components": {
            "schemas": {
                "User": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "bio": {
                            "anyOf": [
                                {"type": "string"},
                                {"type": "object", "properties": {"text": {"type": "string"}}}
                            ]
                        }
                    }
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let user = spec.resolve_ref("#/components/schemas/User").unwrap();
    let resolved = spec.resolve_schema(user);

    assert!(resolved.properties.contains_key("name"));
    // bio is anyOf — the resolver should merge all variant properties
    let bio = resolved.properties.get("bio").unwrap();
    let bio_resolved = spec.resolve_schema(bio);
    // Should have merged properties from both variants
    assert!(bio_resolved.properties.contains_key("text") || bio_resolved.schema_type.is_some());
}

// --- resolve_schema edge cases ---

#[test]
fn test_resolve_schema_broken_ref() {
    // A $ref pointing to a nonexistent schema should return the schema unchanged
    let spec_json = r##"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {},
        "components": {
            "schemas": {
                "Broken": {
                    "type": "object",
                    "properties": {
                        "ref_field": {"$ref": "#/components/schemas/DoesNotExist"}
                    }
                }
            }
        }
    }"##;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let broken = spec.resolve_ref("#/components/schemas/Broken").unwrap();
    let resolved = spec.resolve_schema(broken);
    // Should still have the property, just unresolved
    assert!(resolved.properties.contains_key("ref_field"));
    let ref_field = resolved.properties.get("ref_field").unwrap();
    assert_eq!(
        ref_field.reference,
        Some("#/components/schemas/DoesNotExist".to_string())
    );
}

#[test]
fn test_resolve_ref_invalid_path() {
    // Refs that don't match #/components/schemas/X should return None
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {}
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    assert!(spec.resolve_ref("#/definitions/User").is_none());
    assert!(
        spec.resolve_ref("#/components/responses/NotFound")
            .is_none()
    );
    assert!(spec.resolve_ref("User").is_none());
    assert!(spec.resolve_ref("").is_none());
}

#[test]
fn test_resolve_schema_plain_object_passthrough() {
    // A simple object (no $ref, no allOf/oneOf/anyOf) should be returned as-is
    let schema = Schema {
        schema_type: Some("object".to_string()),
        properties: {
            let mut map = HashMap::new();
            map.insert(
                "id".to_string(),
                Schema {
                    schema_type: Some("string".to_string()),
                    ..Default::default()
                },
            );
            map
        },
        ..Default::default()
    };

    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {}
    }"#;
    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let resolved = spec.resolve_schema(&schema);

    assert_eq!(resolved.schema_type, Some("object".to_string()));
    assert!(resolved.properties.contains_key("id"));
}

#[test]
fn test_resolve_schema_allof_with_ref_and_inline() {
    // Common pattern: allOf combining a $ref with inline properties
    let spec_json = r##"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {
            "/items": {
                "get": {
                    "responses": {
                        "200": {
                            "description": "ok",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "array",
                                        "items": {
                                            "allOf": [
                                                {"$ref": "#/components/schemas/Base"},
                                                {
                                                    "type": "object",
                                                    "properties": {
                                                        "extra": {"type": "string"}
                                                    }
                                                }
                                            ]
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        },
        "components": {
            "schemas": {
                "Base": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "integer"},
                        "name": {"type": "string"}
                    },
                    "required": ["id"]
                }
            }
        }
    }"##;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let endpoints = spec.get_endpoints();
    let schema = endpoints[0].response_schema.as_ref().unwrap();
    let items = schema.items.as_ref().unwrap();
    let resolved = spec.resolve_schema(items);

    assert!(resolved.properties.contains_key("id"));
    assert!(resolved.properties.contains_key("name"));
    assert!(resolved.properties.contains_key("extra"));
    assert!(resolved.required.contains(&"id".to_string()));
}

#[test]
fn test_resolve_schema_oneof_keeps_first_definition() {
    // When two variants define the same property, oneOf should keep the first
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {},
        "components": {
            "schemas": {
                "Poly": {
                    "oneOf": [
                        {
                            "type": "object",
                            "properties": {
                                "status": {"type": "string", "format": "v1"}
                            }
                        },
                        {
                            "type": "object",
                            "properties": {
                                "status": {"type": "string", "format": "v2"}
                            }
                        }
                    ]
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let poly = spec.resolve_ref("#/components/schemas/Poly").unwrap();
    let resolved = spec.resolve_schema(poly);

    // oneOf uses or_insert, so first definition wins
    let status = resolved.properties.get("status").unwrap();
    assert_eq!(status.format, Some("v1".to_string()));
}

#[test]
fn test_resolve_schema_no_components() {
    // Spec with no components section — resolve_ref should return None
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {}
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    assert!(spec.resolve_ref("#/components/schemas/User").is_none());
}

#[test]
fn test_get_response_schema_default_response() {
    // When only "default" response exists (no 200 or 201)
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {
            "/items": {
                "get": {
                    "responses": {
                        "default": {
                            "description": "Default response",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "id": {"type": "string"}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let endpoints = spec.get_endpoints();
    assert_eq!(endpoints.len(), 1);
    let schema = endpoints[0].response_schema.as_ref().unwrap();
    assert!(schema.properties.contains_key("id"));
}

#[test]
fn test_get_response_schema_non_json_content_type() {
    // When content type is not application/json (e.g., application/xml),
    // should still pick up the first available content type
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {
            "/geo": {
                "get": {
                    "responses": {
                        "200": {
                            "description": "ok",
                            "content": {
                                "application/geo+json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "type": {"type": "string"},
                                            "features": {"type": "array"}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let endpoints = spec.get_endpoints();
    assert_eq!(endpoints.len(), 1);
    let schema = endpoints[0].response_schema.as_ref().unwrap();
    assert!(schema.properties.contains_key("type"));
    assert!(schema.properties.contains_key("features"));
}

#[test]
fn test_paths_without_get_include_post() {
    // Paths with POST are now included (POST-for-read support)
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {
            "/items": {
                "get": {
                    "responses": {"200": {"description": "ok"}}
                }
            },
            "/upload": {
                "post": {
                    "responses": {"201": {"description": "created"}}
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let endpoints = spec.get_endpoints();
    assert_eq!(endpoints.len(), 2);
    assert_eq!(endpoints[0].path, "/items");
    assert_eq!(endpoints[0].method, "GET");
    assert_eq!(endpoints[1].path, "/upload");
    assert_eq!(endpoints[1].method, "POST");
}

#[test]
fn test_table_name_deeply_nested_path() {
    let endpoint = EndpointInfo {
        path: "/api/v2/projects/issues/comments".to_string(),
        method: "GET".to_string(),
        response_schema: None,
    };
    assert_eq!(endpoint.table_name(), "api_v2_projects_issues_comments");
}

#[test]
fn test_openapi_31_type_only_null() {
    // Edge case: "type": ["null"] — no actual type, just null
    let schema: Schema = serde_json::from_str(r#"{"type": ["null"]}"#).unwrap();
    assert_eq!(schema.schema_type, None);
    assert!(schema.nullable);
}

#[test]
fn test_openapi_31_type_non_standard_value() {
    // Edge case: "type" is not a string or array (e.g., number or boolean)
    let schema: Schema = serde_json::from_str(r#"{"type": 42}"#).unwrap();
    assert_eq!(schema.schema_type, None);
    assert!(!schema.nullable);
}

#[test]
fn test_schema_no_type() {
    // Schema with no type field at all
    let schema: Schema = serde_json::from_str(r#"{"format": "date-time"}"#).unwrap();
    assert_eq!(schema.schema_type, None);
    assert!(!schema.nullable);
    assert_eq!(schema.format, Some("date-time".to_string()));
}

// --- Fix 1: $ref in Response objects ---

#[test]
fn test_response_ref_resolution() {
    // Response $ref should be resolved via components.responses
    let spec_json = r##"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {
            "/items": {
                "get": {
                    "responses": {
                        "200": {"$ref": "#/components/responses/ItemList"}
                    }
                }
            }
        },
        "components": {
            "responses": {
                "ItemList": {
                    "description": "A list of items",
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "id": {"type": "integer"},
                                        "name": {"type": "string"}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }"##;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let endpoints = spec.get_endpoints();
    assert_eq!(endpoints.len(), 1);
    let schema = endpoints[0].response_schema.as_ref().unwrap();
    let items = schema.items.as_ref().unwrap();
    assert!(items.properties.contains_key("id"));
    assert!(items.properties.contains_key("name"));
}

// --- Fix 2: 2XX wildcard status codes ---

#[test]
fn test_response_schema_from_2xx_wildcard() {
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {
            "/items": {
                "get": {
                    "responses": {
                        "2XX": {
                            "description": "Success",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "id": {"type": "string"},
                                            "status": {"type": "string"}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let endpoints = spec.get_endpoints();
    assert_eq!(endpoints.len(), 1);
    let schema = endpoints[0].response_schema.as_ref().unwrap();
    assert!(schema.properties.contains_key("id"));
    assert!(schema.properties.contains_key("status"));
}

#[test]
fn test_200_preferred_over_2xx() {
    // "200" should be preferred over "2XX"
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {
            "/items": {
                "get": {
                    "responses": {
                        "200": {
                            "description": "OK",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "from_200": {"type": "string"}
                                        }
                                    }
                                }
                            }
                        },
                        "2XX": {
                            "description": "Wildcard",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "from_2xx": {"type": "string"}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let endpoints = spec.get_endpoints();
    let schema = endpoints[0].response_schema.as_ref().unwrap();
    assert!(schema.properties.contains_key("from_200"));
    assert!(!schema.properties.contains_key("from_2xx"));
}

// --- Fix 3: writeOnly properties ---

#[test]
fn test_write_only_property_deserialization() {
    let schema: Schema = serde_json::from_str(r#"{"type": "string", "writeOnly": true}"#).unwrap();
    assert!(schema.write_only);
    assert_eq!(schema.schema_type, Some("string".to_string()));
}

#[test]
fn test_write_only_default_false() {
    let schema: Schema = serde_json::from_str(r#"{"type": "string"}"#).unwrap();
    assert!(!schema.write_only);
}

// --- Fix 4: Multi-type arrays ---

#[test]
fn test_multi_type_array_becomes_none() {
    // ["string", "integer"] — multiple non-null types → schema_type = None (jsonb)
    let schema: Schema = serde_json::from_str(r#"{"type": ["string", "integer"]}"#).unwrap();
    assert_eq!(schema.schema_type, None);
    assert!(!schema.nullable);
}

#[test]
fn test_multi_type_array_with_null() {
    // ["string", "integer", "null"] → None type + nullable
    let schema: Schema =
        serde_json::from_str(r#"{"type": ["string", "integer", "null"]}"#).unwrap();
    assert_eq!(schema.schema_type, None);
    assert!(schema.nullable);
}

#[test]
fn test_single_type_array_still_works() {
    // ["string", "null"] → exactly one non-null type → Some("string")
    let schema: Schema = serde_json::from_str(r#"{"type": ["string", "null"]}"#).unwrap();
    assert_eq!(schema.schema_type, Some("string".to_string()));
    assert!(schema.nullable);
}

// --- Fix 5: Composition on primitives ---

#[test]
fn test_oneof_primitives_not_object() {
    // oneOf with primitive types should NOT produce schema_type = "object"
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {},
        "components": {
            "schemas": {
                "StringOrInt": {
                    "oneOf": [
                        {"type": "string"},
                        {"type": "integer"}
                    ]
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let schema = spec
        .resolve_ref("#/components/schemas/StringOrInt")
        .unwrap();
    let resolved = spec.resolve_schema(schema);
    // Should be None (→ jsonb), NOT "object"
    assert_eq!(resolved.schema_type, None);
    assert!(resolved.properties.is_empty());
}

#[test]
fn test_oneof_with_objects_stays_object() {
    // oneOf with object schemas should still produce "object"
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {},
        "components": {
            "schemas": {
                "UserOrOrg": {
                    "oneOf": [
                        {
                            "type": "object",
                            "properties": {"user_id": {"type": "string"}}
                        },
                        {
                            "type": "object",
                            "properties": {"org_id": {"type": "string"}}
                        }
                    ]
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let schema = spec.resolve_ref("#/components/schemas/UserOrOrg").unwrap();
    let resolved = spec.resolve_schema(schema);
    assert_eq!(resolved.schema_type, Some("object".to_string()));
    assert!(resolved.properties.contains_key("user_id"));
    assert!(resolved.properties.contains_key("org_id"));
}

// --- Fix 7: Server URL variable substitution ---

#[test]
fn test_server_variable_substitution() {
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "servers": [
            {
                "url": "https://{region}.api.example.com/v{version}",
                "variables": {
                    "region": {"default": "us-east-1"},
                    "version": {"default": "2"}
                }
            }
        ],
        "paths": {}
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    assert_eq!(
        spec.base_url(),
        Some("https://us-east-1.api.example.com/v2".to_string())
    );
}

#[test]
fn test_server_no_variables() {
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "servers": [{"url": "https://api.example.com"}],
        "paths": {}
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    assert_eq!(spec.base_url(), Some("https://api.example.com".to_string()));
}

#[test]
fn test_endpoints_sorted_alphabetically() {
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {
            "/zebras": {"get": {"responses": {"200": {"description": "ok"}}}},
            "/apples": {"get": {"responses": {"200": {"description": "ok"}}}},
            "/middle": {"get": {"responses": {"200": {"description": "ok"}}}}
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let endpoints = spec.get_endpoints();
    assert_eq!(endpoints.len(), 3);
    assert_eq!(endpoints[0].path, "/apples");
    assert_eq!(endpoints[1].path, "/middle");
    assert_eq!(endpoints[2].path, "/zebras");
}

#[test]
fn test_response_schema_charset_content_type() {
    // Content type with charset parameter should still match
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {
            "/items": {
                "get": {
                    "responses": {
                        "200": {
                            "description": "ok",
                            "content": {
                                "application/json; charset=utf-8": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "id": {"type": "integer"}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let endpoints = spec.get_endpoints();
    assert_eq!(endpoints.len(), 1);
    let schema = endpoints[0].response_schema.as_ref().unwrap();
    assert!(schema.properties.contains_key("id"));
}

#[test]
fn test_response_schema_json_preferred_over_xml() {
    // When both JSON and XML content types exist, JSON should be preferred
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {
            "/items": {
                "get": {
                    "responses": {
                        "200": {
                            "description": "ok",
                            "content": {
                                "application/xml": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "xml_field": {"type": "string"}
                                        }
                                    }
                                },
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "json_field": {"type": "string"}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let endpoints = spec.get_endpoints();
    assert_eq!(endpoints.len(), 1);
    let schema = endpoints[0].response_schema.as_ref().unwrap();
    assert!(schema.properties.contains_key("json_field"));
}

#[test]
fn test_resolve_schema_ref_with_nullable_sibling() {
    // OpenAPI 3.1: $ref with nullable sibling should merge
    let spec_json = r#"{
        "openapi": "3.1.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {},
        "components": {
            "schemas": {
                "Address": {
                    "type": "object",
                    "properties": {
                        "street": {"type": "string"},
                        "city": {"type": "string"}
                    }
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();

    let ref_with_nullable = Schema {
        reference: Some("#/components/schemas/Address".to_string()),
        nullable: true,
        ..Default::default()
    };

    let resolved = spec.resolve_schema(&ref_with_nullable);
    assert!(resolved.nullable);
    assert!(resolved.properties.contains_key("street"));
    assert!(resolved.properties.contains_key("city"));
}

#[test]
fn test_resolve_schema_ref_with_extra_properties() {
    // OpenAPI 3.1: $ref with additional properties sibling should merge
    let spec_json = r#"{
        "openapi": "3.1.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {},
        "components": {
            "schemas": {
                "Base": {
                    "type": "object",
                    "required": ["id"],
                    "properties": {
                        "id": {"type": "integer"},
                        "name": {"type": "string"}
                    }
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();

    let mut extra_props = std::collections::HashMap::new();
    extra_props.insert(
        "extra_field".to_string(),
        Schema {
            schema_type: Some("string".to_string()),
            ..Default::default()
        },
    );

    let ref_with_props = Schema {
        reference: Some("#/components/schemas/Base".to_string()),
        properties: extra_props,
        required: vec!["extra_field".to_string()],
        ..Default::default()
    };

    let resolved = spec.resolve_schema(&ref_with_props);
    assert!(resolved.properties.contains_key("id"));
    assert!(resolved.properties.contains_key("name"));
    assert!(resolved.properties.contains_key("extra_field"));
    assert!(resolved.required.contains(&"id".to_string()));
    assert!(resolved.required.contains(&"extra_field".to_string()));
}

// --- POST-for-read tests ---

#[test]
fn test_post_endpoint_included() {
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {
            "/search": {
                "post": {
                    "responses": {
                        "200": {
                            "description": "ok",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "array",
                                        "items": {
                                            "type": "object",
                                            "properties": {
                                                "id": {"type": "string"}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let endpoints = spec.get_endpoints();
    assert_eq!(endpoints.len(), 1);
    assert_eq!(endpoints[0].path, "/search");
    assert_eq!(endpoints[0].method, "POST");
    assert!(endpoints[0].response_schema.is_some());
}

#[test]
fn test_post_endpoint_table_name_suffix() {
    let endpoint = EndpointInfo {
        path: "/search".to_string(),
        method: "POST".to_string(),
        response_schema: None,
    };
    assert_eq!(endpoint.table_name(), "search_post");

    let get_endpoint = EndpointInfo {
        path: "/search".to_string(),
        method: "GET".to_string(),
        response_schema: None,
    };
    assert_eq!(get_endpoint.table_name(), "search");
}

#[test]
fn test_get_and_post_same_path() {
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {
            "/items": {
                "get": {
                    "responses": {"200": {"description": "ok"}}
                },
                "post": {
                    "responses": {"200": {"description": "ok"}}
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let endpoints = spec.get_endpoints();
    assert_eq!(endpoints.len(), 2);
    // Sorted by path then method, so GET comes first
    assert_eq!(endpoints[0].method, "GET");
    assert_eq!(endpoints[1].method, "POST");
    // Table names should differ
    let names: Vec<String> = endpoints.iter().map(|e| e.table_name()).collect();
    assert_eq!(names[0], "items");
    assert_eq!(names[1], "items_post");
}

#[test]
fn test_parameterized_post_excluded() {
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {
            "/items/{id}/search": {
                "post": {
                    "responses": {"200": {"description": "ok"}}
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let endpoints = spec.get_endpoints();
    assert_eq!(endpoints.len(), 0);
}

#[test]
fn test_post_only_path() {
    // Path with only POST and no GET should still be included
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {
            "/search": {
                "post": {
                    "responses": {"200": {"description": "ok"}}
                }
            },
            "/items": {
                "get": {
                    "responses": {"200": {"description": "ok"}}
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let endpoints = spec.get_endpoints();
    assert_eq!(endpoints.len(), 2);
    assert!(
        endpoints
            .iter()
            .any(|e| e.method == "GET" && e.path == "/items")
    );
    assert!(
        endpoints
            .iter()
            .any(|e| e.method == "POST" && e.path == "/search")
    );
}

// --- OpenAPI 3.1 real-world API pattern tests ---

#[test]
fn test_stripe_expandable_anyof() {
    // Stripe pattern: anyOf: [{type: "string"}, {$ref: "..."}] for expandable ID/object fields
    let spec_json = r##"{
        "openapi": "3.1.0",
        "info": {"title": "Stripe", "version": "1.0"},
        "paths": {},
        "components": {
            "schemas": {
                "Customer": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"},
                        "name": {"type": "string"}
                    }
                },
                "Charge": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"},
                        "customer": {
                            "anyOf": [
                                {"type": "string"},
                                {"$ref": "#/components/schemas/Customer"}
                            ]
                        }
                    }
                }
            }
        }
    }"##;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let charge = spec.resolve_ref("#/components/schemas/Charge").unwrap();
    let resolved = spec.resolve_schema(charge);

    assert!(resolved.properties.contains_key("id"));
    assert!(resolved.properties.contains_key("customer"));
    // customer is anyOf → resolver merges; the Customer variant has properties (id, name),
    // so merged schema_type = "object". The string variant has no properties.
    let customer = resolved.properties.get("customer").unwrap();
    let customer_resolved = spec.resolve_schema(customer);
    // Should have merged properties from the Customer $ref
    assert!(
        customer_resolved.properties.contains_key("id") || customer_resolved.schema_type.is_some()
    );
}

#[test]
fn test_github_nullable_anyof_null_type() {
    // GitHub 3.1 pattern: anyOf: [{$ref: "..."}, {type: "null"}] for nullable refs
    let spec_json = r##"{
        "openapi": "3.1.0",
        "info": {"title": "GitHub", "version": "1.0"},
        "paths": {},
        "components": {
            "schemas": {
                "SimpleUser": {
                    "type": "object",
                    "properties": {
                        "login": {"type": "string"},
                        "id": {"type": "integer"}
                    }
                },
                "Issue": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "integer"},
                        "assignee": {
                            "anyOf": [
                                {"$ref": "#/components/schemas/SimpleUser"},
                                {"type": "null"}
                            ]
                        }
                    }
                }
            }
        }
    }"##;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let issue = spec.resolve_ref("#/components/schemas/Issue").unwrap();
    let resolved = spec.resolve_schema(issue);

    assert!(resolved.properties.contains_key("assignee"));
    let assignee = resolved.properties.get("assignee").unwrap();
    let assignee_resolved = spec.resolve_schema(assignee);
    // Should have merged SimpleUser properties
    assert!(assignee_resolved.properties.contains_key("login"));
    assert!(assignee_resolved.properties.contains_key("id"));
}

#[test]
fn test_kubernetes_deep_ref_chain_8_levels() {
    // Kubernetes-style: 8-level chain of $ref → $ref → ... deep resolution
    let spec_json = r##"{
        "openapi": "3.0.0",
        "info": {"title": "K8s", "version": "1.0"},
        "paths": {},
        "components": {
            "schemas": {
                "L1": {"$ref": "#/components/schemas/L2"},
                "L2": {"$ref": "#/components/schemas/L3"},
                "L3": {"$ref": "#/components/schemas/L4"},
                "L4": {"$ref": "#/components/schemas/L5"},
                "L5": {"$ref": "#/components/schemas/L6"},
                "L6": {"$ref": "#/components/schemas/L7"},
                "L7": {"$ref": "#/components/schemas/L8"},
                "L8": {
                    "type": "object",
                    "properties": {
                        "value": {"type": "string"}
                    }
                }
            }
        }
    }"##;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let l1 = spec.resolve_ref("#/components/schemas/L1").unwrap();
    let resolved = spec.resolve_schema(l1);

    // Should resolve through all 8 levels
    assert!(resolved.properties.contains_key("value"));
    assert_eq!(
        resolved.properties.get("value").unwrap().schema_type,
        Some("string".to_string())
    );
}

#[test]
fn test_allof_multiple_refs() {
    // Multi-inheritance: allOf: [{$ref: "A"}, {$ref: "B"}, {inline}]
    let spec_json = r##"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {},
        "components": {
            "schemas": {
                "Auditable": {
                    "type": "object",
                    "properties": {
                        "created_at": {"type": "string", "format": "date-time"},
                        "updated_at": {"type": "string", "format": "date-time"}
                    }
                },
                "Identifiable": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"},
                        "slug": {"type": "string"}
                    },
                    "required": ["id"]
                },
                "Resource": {
                    "allOf": [
                        {"$ref": "#/components/schemas/Identifiable"},
                        {"$ref": "#/components/schemas/Auditable"},
                        {
                            "type": "object",
                            "properties": {
                                "name": {"type": "string"},
                                "status": {"type": "string"}
                            },
                            "required": ["name"]
                        }
                    ]
                }
            }
        }
    }"##;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let resource = spec.resolve_ref("#/components/schemas/Resource").unwrap();
    let resolved = spec.resolve_schema(resource);

    // Should have properties from all three sources
    assert!(resolved.properties.contains_key("id"));
    assert!(resolved.properties.contains_key("slug"));
    assert!(resolved.properties.contains_key("created_at"));
    assert!(resolved.properties.contains_key("updated_at"));
    assert!(resolved.properties.contains_key("name"));
    assert!(resolved.properties.contains_key("status"));

    // Required from both Identifiable and inline
    assert!(resolved.required.contains(&"id".to_string()));
    assert!(resolved.required.contains(&"name".to_string()));
}

#[test]
fn test_openapi_31_nullable_array() {
    // GitHub pattern: type: ["array", "null"] with items
    let schema: Schema =
        serde_json::from_str(r#"{"type": ["array", "null"], "items": {"type": "string"}}"#)
            .unwrap();
    assert_eq!(schema.schema_type, Some("array".to_string()));
    assert!(schema.nullable);
    assert!(schema.items.is_some());
    assert_eq!(
        schema.items.as_ref().unwrap().schema_type,
        Some("string".to_string())
    );
}

#[test]
fn test_openapi_31_nullable_boolean() {
    // General 3.1 pattern: type: ["boolean", "null"]
    let schema: Schema = serde_json::from_str(r#"{"type": ["boolean", "null"]}"#).unwrap();
    assert_eq!(schema.schema_type, Some("boolean".to_string()));
    assert!(schema.nullable);
}

#[test]
fn test_type_array_three_non_null_types() {
    // Edge case: type: ["string", "integer", "boolean"] → None (jsonb)
    let schema: Schema =
        serde_json::from_str(r#"{"type": ["string", "integer", "boolean"]}"#).unwrap();
    assert_eq!(schema.schema_type, None);
    assert!(!schema.nullable);
}

#[test]
fn test_content_type_jsonld() {
    // NWS/JSON-LD: application/ld+json picked up via fallback
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "NWS", "version": "1.0"},
        "paths": {
            "/alerts": {
                "get": {
                    "responses": {
                        "200": {
                            "description": "ok",
                            "content": {
                                "application/ld+json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "@context": {"type": "object"},
                                            "@graph": {"type": "array", "items": {"type": "object"}}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let endpoints = spec.get_endpoints();
    assert_eq!(endpoints.len(), 1);
    let schema = endpoints[0].response_schema.as_ref().unwrap();
    assert!(schema.properties.contains_key("@graph"));
}

#[test]
fn test_content_type_jsonapi() {
    // JSON:API: application/vnd.api+json picked up via fallback
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "JSON:API", "version": "1.0"},
        "paths": {
            "/articles": {
                "get": {
                    "responses": {
                        "200": {
                            "description": "ok",
                            "content": {
                                "application/vnd.api+json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "data": {"type": "array"},
                                            "meta": {"type": "object"}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let endpoints = spec.get_endpoints();
    assert_eq!(endpoints.len(), 1);
    let schema = endpoints[0].response_schema.as_ref().unwrap();
    assert!(schema.properties.contains_key("data"));
    assert!(schema.properties.contains_key("meta"));
}

#[test]
fn test_ref_with_description_sibling() {
    // Common 3.1 pattern: $ref + description sibling (description ignored, ref resolved)
    let spec_json = r##"{
        "openapi": "3.1.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {},
        "components": {
            "schemas": {
                "Pet": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "tag": {"type": "string"}
                    }
                }
            }
        }
    }"##;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();

    // Simulate a $ref with a description sibling — description is not in Schema struct,
    // so it's implicitly ignored. The key point is $ref still resolves correctly.
    let ref_schema = Schema {
        reference: Some("#/components/schemas/Pet".to_string()),
        ..Default::default()
    };

    let resolved = spec.resolve_schema(&ref_schema);
    assert!(resolved.properties.contains_key("name"));
    assert!(resolved.properties.contains_key("tag"));
}

#[test]
fn test_stripe_metadata_additional_properties() {
    // Stripe pattern: additionalProperties without properties → maps to jsonb (type: "object")
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Stripe", "version": "1.0"},
        "paths": {
            "/charges": {
                "get": {
                    "responses": {
                        "200": {
                            "description": "ok",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "id": {"type": "string"},
                                            "metadata": {
                                                "type": "object"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let endpoints = spec.get_endpoints();
    let schema = endpoints[0].response_schema.as_ref().unwrap();
    let metadata = schema.properties.get("metadata").unwrap();
    // type: "object" with no properties → maps to jsonb
    assert_eq!(metadata.schema_type, Some("object".to_string()));
    assert!(metadata.properties.is_empty());
}

#[test]
fn test_discriminator_doesnt_break_parsing() {
    // Polymorphic APIs: discriminator field on oneOf is silently ignored
    let spec_json = r##"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {},
        "components": {
            "schemas": {
                "Cat": {
                    "type": "object",
                    "properties": {
                        "pet_type": {"type": "string"},
                        "purrs": {"type": "boolean"}
                    }
                },
                "Dog": {
                    "type": "object",
                    "properties": {
                        "pet_type": {"type": "string"},
                        "barks": {"type": "boolean"}
                    }
                },
                "Pet": {
                    "oneOf": [
                        {"$ref": "#/components/schemas/Cat"},
                        {"$ref": "#/components/schemas/Dog"}
                    ],
                    "discriminator": {
                        "propertyName": "pet_type"
                    }
                }
            }
        }
    }"##;

    // Should parse without error (discriminator field is ignored by serde)
    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let pet = spec.resolve_ref("#/components/schemas/Pet").unwrap();
    let resolved = spec.resolve_schema(pet);

    // oneOf merges all variant properties as nullable
    assert!(resolved.properties.contains_key("pet_type"));
    assert!(resolved.properties.contains_key("purrs"));
    assert!(resolved.properties.contains_key("barks"));
}

#[test]
fn test_empty_type_array() {
    // Edge case: type: [] → None (jsonb)
    let schema: Schema = serde_json::from_str(r#"{"type": []}"#).unwrap();
    assert_eq!(schema.schema_type, None);
    assert!(!schema.nullable);
}

#[test]
fn test_allof_with_properties_sibling() {
    // K8s-style: allOf + sibling properties → allOf takes priority (sibling properties
    // are not in Schema's allOf path, they stay on the outer schema)
    let spec_json = r##"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {},
        "components": {
            "schemas": {
                "Base": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"}
                    }
                },
                "Extended": {
                    "allOf": [
                        {"$ref": "#/components/schemas/Base"},
                        {
                            "type": "object",
                            "properties": {
                                "name": {"type": "string"}
                            }
                        }
                    ],
                    "properties": {
                        "sibling_prop": {"type": "boolean"}
                    }
                }
            }
        }
    }"##;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let extended = spec.resolve_ref("#/components/schemas/Extended").unwrap();
    let resolved = spec.resolve_schema(extended);

    // allOf properties should be present
    assert!(resolved.properties.contains_key("id"));
    assert!(resolved.properties.contains_key("name"));
    // Sibling properties from the outer schema are not included in allOf resolution
    // (they live on the unresolvedschema, not merged by resolve_schema)
}

#[test]
fn test_response_only_error_codes() {
    // Only 4xx/5xx responses → None schema (no success response to extract)
    let spec_json = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Error API", "version": "1.0"},
        "paths": {
            "/errors": {
                "get": {
                    "responses": {
                        "400": {
                            "description": "Bad request",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "error": {"type": "string"}
                                        }
                                    }
                                }
                            }
                        },
                        "500": {
                            "description": "Internal error"
                        }
                    }
                }
            }
        }
    }"#;

    let spec = OpenApiSpec::from_str(spec_json).unwrap();
    let endpoints = spec.get_endpoints();
    assert_eq!(endpoints.len(), 1);
    // No 200/201/2XX/default → response_schema should be None
    assert!(endpoints[0].response_schema.is_none());
}
