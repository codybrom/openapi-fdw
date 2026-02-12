use super::*;

#[test]
fn test_sanitize_column_name() {
    assert_eq!(sanitize_column_name("userName"), "user_name");
    assert_eq!(sanitize_column_name("user-name"), "user_name");
    assert_eq!(sanitize_column_name("123abc"), "_123abc");
    assert_eq!(sanitize_column_name("already_snake"), "already_snake");
}

#[test]
fn test_sanitize_column_name_acronyms() {
    // Consecutive uppercase letters (acronyms) should be grouped
    assert_eq!(sanitize_column_name("clusterIP"), "cluster_ip");
    assert_eq!(sanitize_column_name("HTMLParser"), "html_parser");
    assert_eq!(sanitize_column_name("getHTTPSUrl"), "get_https_url");
    assert_eq!(sanitize_column_name("IOError"), "io_error");
    assert_eq!(sanitize_column_name("apiURL"), "api_url");
    // Single uppercase still works
    assert_eq!(sanitize_column_name("firstName"), "first_name");
}

#[test]
fn test_sanitize_column_name_special_chars() {
    // @ prefix (JSON-LD)
    assert_eq!(sanitize_column_name("@id"), "_id");
    assert_eq!(sanitize_column_name("@type"), "_type");
    // Dots (nested keys)
    assert_eq!(sanitize_column_name("user.name"), "user_name");
    // Plus/minus (GitHub reactions)
    assert_eq!(sanitize_column_name("+1"), "_1");
    assert_eq!(sanitize_column_name("-1"), "_1");
}

#[test]
fn test_openapi_to_pg_type() {
    let spec = OpenApiSpec::from_str(
        r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test"},
        "paths": {}
    }"#,
    )
    .unwrap();

    let string_schema = Schema {
        schema_type: Some("string".to_string()),
        ..Default::default()
    };
    assert_eq!(openapi_to_pg_type(&string_schema, &spec), "text");

    let date_schema = Schema {
        schema_type: Some("string".to_string()),
        format: Some("date-time".to_string()),
        ..Default::default()
    };
    assert_eq!(openapi_to_pg_type(&date_schema, &spec), "timestamptz");

    let int_schema = Schema {
        schema_type: Some("integer".to_string()),
        format: Some("int32".to_string()),
        ..Default::default()
    };
    assert_eq!(openapi_to_pg_type(&int_schema, &spec), "integer");
}

#[test]
fn test_openapi_to_pg_type_unix_time() {
    let spec = OpenApiSpec::from_str(
        r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test"},
        "paths": {}
    }"#,
    )
    .unwrap();

    // Stripe's format: "unix-time" should map to timestamptz
    let unix_time_schema = Schema {
        schema_type: Some("integer".to_string()),
        format: Some("unix-time".to_string()),
        ..Default::default()
    };
    assert_eq!(openapi_to_pg_type(&unix_time_schema, &spec), "timestamptz");

    // Regular integer without format should still be bigint
    let int_schema = Schema {
        schema_type: Some("integer".to_string()),
        ..Default::default()
    };
    assert_eq!(openapi_to_pg_type(&int_schema, &spec), "bigint");

    // string format: "date" should be date
    let date_schema = Schema {
        schema_type: Some("string".to_string()),
        format: Some("date".to_string()),
        ..Default::default()
    };
    assert_eq!(openapi_to_pg_type(&date_schema, &spec), "date");

    // boolean
    let bool_schema = Schema {
        schema_type: Some("boolean".to_string()),
        ..Default::default()
    };
    assert_eq!(openapi_to_pg_type(&bool_schema, &spec), "boolean");

    // number format: "float" → real
    let float_schema = Schema {
        schema_type: Some("number".to_string()),
        format: Some("float".to_string()),
        ..Default::default()
    };
    assert_eq!(openapi_to_pg_type(&float_schema, &spec), "real");

    // number without format → double precision
    let num_schema = Schema {
        schema_type: Some("number".to_string()),
        ..Default::default()
    };
    assert_eq!(openapi_to_pg_type(&num_schema, &spec), "double precision");

    // array → jsonb
    let arr_schema = Schema {
        schema_type: Some("array".to_string()),
        ..Default::default()
    };
    assert_eq!(openapi_to_pg_type(&arr_schema, &spec), "jsonb");

    // object → jsonb
    let obj_schema = Schema {
        schema_type: Some("object".to_string()),
        ..Default::default()
    };
    assert_eq!(openapi_to_pg_type(&obj_schema, &spec), "jsonb");

    // None type → jsonb (OpenAPI 3.1 type arrays that resolve to None)
    let none_schema = Schema {
        schema_type: None,
        ..Default::default()
    };
    assert_eq!(openapi_to_pg_type(&none_schema, &spec), "jsonb");
}

#[test]
fn test_openapi_to_pg_type_time_format() {
    let spec = OpenApiSpec::from_str(
        r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test"},
        "paths": {}
    }"#,
    )
    .unwrap();

    let time_schema = Schema {
        schema_type: Some("string".to_string()),
        format: Some("time".to_string()),
        ..Default::default()
    };
    assert_eq!(openapi_to_pg_type(&time_schema, &spec), "time");
}

#[test]
fn test_openapi_to_pg_type_byte_binary_format() {
    let spec = OpenApiSpec::from_str(
        r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test"},
        "paths": {}
    }"#,
    )
    .unwrap();

    let byte_schema = Schema {
        schema_type: Some("string".to_string()),
        format: Some("byte".to_string()),
        ..Default::default()
    };
    assert_eq!(openapi_to_pg_type(&byte_schema, &spec), "bytea");

    let binary_schema = Schema {
        schema_type: Some("string".to_string()),
        format: Some("binary".to_string()),
        ..Default::default()
    };
    assert_eq!(openapi_to_pg_type(&binary_schema, &spec), "bytea");
}

#[test]
fn test_openapi_to_pg_type_uuid_format() {
    let spec = OpenApiSpec::from_str(
        r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test"},
        "paths": {}
    }"#,
    )
    .unwrap();

    let uuid_schema = Schema {
        schema_type: Some("string".to_string()),
        format: Some("uuid".to_string()),
        ..Default::default()
    };
    assert_eq!(openapi_to_pg_type(&uuid_schema, &spec), "uuid");
}

#[test]
fn test_column_name_collision_dedup() {
    // Properties that collide after sanitization should get suffixed
    let spec = OpenApiSpec::from_str(
        r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test"},
        "paths": {}
    }"#,
    )
    .unwrap();

    let mut properties = HashMap::new();
    properties.insert(
        "user-name".to_string(),
        Schema {
            schema_type: Some("string".to_string()),
            ..Default::default()
        },
    );
    properties.insert(
        "userName".to_string(),
        Schema {
            schema_type: Some("string".to_string()),
            ..Default::default()
        },
    );

    let schema = Schema {
        schema_type: Some("object".to_string()),
        properties,
        ..Default::default()
    };

    let columns = extract_columns(&schema, &spec, false);
    let names: Vec<&str> = columns.iter().map(|c| c.name.as_str()).collect();

    // Both should exist, one with a suffix
    assert!(
        names.contains(&"user_name"),
        "Expected user_name in {names:?}",
    );
    assert!(
        names.contains(&"user_name_1"),
        "Expected user_name_1 for collision in {names:?}",
    );
}

// --- generate_all_tables filter tests ---

fn make_test_spec() -> OpenApiSpec {
    OpenApiSpec::from_str(
        r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0"},
        "paths": {
            "/users": {
                "get": {
                    "responses": {"200": {"description": "ok", "content": {"application/json": {"schema": {"type": "array", "items": {"type": "object", "properties": {"id": {"type": "string"}}}}}}}}
                }
            },
            "/posts": {
                "get": {
                    "responses": {"200": {"description": "ok", "content": {"application/json": {"schema": {"type": "array", "items": {"type": "object", "properties": {"id": {"type": "string"}}}}}}}}
                }
            },
            "/comments": {
                "get": {
                    "responses": {"200": {"description": "ok", "content": {"application/json": {"schema": {"type": "array", "items": {"type": "object", "properties": {"id": {"type": "string"}}}}}}}}
                }
            }
        }
    }"#,
    )
    .unwrap()
}

#[test]
fn test_generate_all_tables_no_filter() {
    let spec = make_test_spec();
    let tables = generate_all_tables(&spec, "test_server", None, false, false);
    assert_eq!(tables.len(), 3);
}

#[test]
fn test_generate_all_tables_limit_to() {
    let spec = make_test_spec();
    let filter = vec!["users".to_string(), "posts".to_string()];
    let tables = generate_all_tables(&spec, "test_server", Some(&filter), false, false);
    assert_eq!(tables.len(), 2);
    assert!(tables.iter().any(|t| t.contains("\"users\"")));
    assert!(tables.iter().any(|t| t.contains("\"posts\"")));
    assert!(!tables.iter().any(|t| t.contains("\"comments\"")));
}

#[test]
fn test_generate_all_tables_except() {
    let spec = make_test_spec();
    let filter = vec!["comments".to_string()];
    let tables = generate_all_tables(&spec, "test_server", Some(&filter), true, false);
    assert_eq!(tables.len(), 2);
    assert!(tables.iter().any(|t| t.contains("\"users\"")));
    assert!(tables.iter().any(|t| t.contains("\"posts\"")));
    assert!(!tables.iter().any(|t| t.contains("\"comments\"")));
}

#[test]
fn test_generate_all_tables_limit_to_nonexistent() {
    let spec = make_test_spec();
    let filter = vec!["nonexistent".to_string()];
    let tables = generate_all_tables(&spec, "test_server", Some(&filter), false, false);
    assert_eq!(tables.len(), 0);
}

#[test]
fn test_generate_all_tables_include_attrs() {
    let spec = make_test_spec();
    let tables = generate_all_tables(&spec, "test_server", None, false, true);
    // All tables should have an 'attrs' column
    for table in &tables {
        assert!(
            table.contains("\"attrs\" jsonb"),
            "Missing attrs in: {table}"
        );
    }
}

#[test]
fn test_generate_all_tables_exclude_attrs() {
    let spec = make_test_spec();
    let tables = generate_all_tables(&spec, "test_server", None, false, false);
    // No table should have an 'attrs' column
    for table in &tables {
        assert!(!table.contains("\"attrs\""), "Unexpected attrs in: {table}");
    }
}

#[test]
fn test_generate_foreign_table_no_schema() {
    // Endpoint with no response schema → default id + attrs columns
    let spec = make_test_spec();
    let endpoint = crate::spec::EndpointInfo {
        path: "/health".to_string(),
        method: "GET".to_string(),
        response_schema: None,
    };
    let table = generate_foreign_table(&endpoint, &spec, "test_server", true);
    assert!(table.contains("\"id\" text NOT NULL"));
    assert!(table.contains("\"attrs\" jsonb"));
    assert!(table.contains("rowid_column 'id'"));
}

#[test]
fn test_write_only_properties_filtered() {
    let spec = OpenApiSpec::from_str(
        r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test"},
        "paths": {}
    }"#,
    )
    .unwrap();

    let mut properties = HashMap::new();
    properties.insert(
        "username".to_string(),
        crate::spec::Schema {
            schema_type: Some("string".to_string()),
            ..Default::default()
        },
    );
    properties.insert(
        "password".to_string(),
        crate::spec::Schema {
            schema_type: Some("string".to_string()),
            write_only: true,
            ..Default::default()
        },
    );
    properties.insert(
        "email".to_string(),
        crate::spec::Schema {
            schema_type: Some("string".to_string()),
            ..Default::default()
        },
    );

    let schema = crate::spec::Schema {
        schema_type: Some("object".to_string()),
        properties,
        ..Default::default()
    };

    let columns = extract_columns(&schema, &spec, false);
    let names: Vec<&str> = columns.iter().map(|c| c.name.as_str()).collect();

    assert!(names.contains(&"username"), "username should be included");
    assert!(names.contains(&"email"), "email should be included");
    assert!(
        !names.contains(&"password"),
        "password (writeOnly) should be excluded"
    );
    assert_eq!(columns.len(), 2);
}

#[test]
fn test_generate_foreign_table_post_method() {
    let spec = make_test_spec();
    let endpoint = crate::spec::EndpointInfo {
        path: "/search".to_string(),
        method: "POST".to_string(),
        response_schema: None,
    };
    let table = generate_foreign_table(&endpoint, &spec, "test_server", true);
    assert!(
        table.contains("method 'POST'"),
        "POST DDL should include method option: {table}"
    );
    assert!(table.contains("endpoint '/search'"));
}

#[test]
fn test_generate_foreign_table_get_no_method() {
    let spec = make_test_spec();
    let endpoint = crate::spec::EndpointInfo {
        path: "/items".to_string(),
        method: "GET".to_string(),
        response_schema: None,
    };
    let table = generate_foreign_table(&endpoint, &spec, "test_server", true);
    assert!(
        !table.contains("method "),
        "GET DDL should NOT include method option: {table}"
    );
    assert!(table.contains("endpoint '/items'"));
}

// --- OpenAPI 3.1 type mapping and DDL generation tests ---

#[test]
fn test_int64_format_explicit() {
    // integer + format: "int64" → bigint
    let spec =
        OpenApiSpec::from_str(r#"{"openapi": "3.0.0", "info": {"title": "T"}, "paths": {}}"#)
            .unwrap();

    let schema = crate::spec::Schema {
        schema_type: Some("integer".to_string()),
        format: Some("int64".to_string()),
        ..Default::default()
    };
    assert_eq!(openapi_to_pg_type(&schema, &spec), "bigint");
}

#[test]
fn test_double_format_explicit() {
    // number + format: "double" → double precision
    let spec =
        OpenApiSpec::from_str(r#"{"openapi": "3.0.0", "info": {"title": "T"}, "paths": {}}"#)
            .unwrap();

    let schema = crate::spec::Schema {
        schema_type: Some("number".to_string()),
        format: Some("double".to_string()),
        ..Default::default()
    };
    assert_eq!(openapi_to_pg_type(&schema, &spec), "double precision");
}

#[test]
fn test_unknown_string_format_fallback() {
    // Unknown string formats (email, uri, hostname, ipv4, ipv6, password) → text
    let spec =
        OpenApiSpec::from_str(r#"{"openapi": "3.0.0", "info": {"title": "T"}, "paths": {}}"#)
            .unwrap();

    for fmt in &["email", "uri", "hostname", "ipv4", "ipv6", "password"] {
        let schema = crate::spec::Schema {
            schema_type: Some("string".to_string()),
            format: Some(fmt.to_string()),
            ..Default::default()
        };
        assert_eq!(
            openapi_to_pg_type(&schema, &spec),
            "text",
            "format '{fmt}' should map to text",
        );
    }
}

#[test]
fn test_extract_columns_github_type_arrays() {
    // Nullable via type arrays in column extraction (OpenAPI 3.1)
    let spec =
        OpenApiSpec::from_str(r#"{"openapi": "3.1.0", "info": {"title": "T"}, "paths": {}}"#)
            .unwrap();

    let mut properties = HashMap::new();
    properties.insert(
        "name".to_string(),
        crate::spec::Schema {
            schema_type: Some("string".to_string()),
            nullable: true, // from type: ["string", "null"]
            ..Default::default()
        },
    );
    properties.insert(
        "count".to_string(),
        crate::spec::Schema {
            schema_type: Some("integer".to_string()),
            ..Default::default()
        },
    );

    let schema = crate::spec::Schema {
        schema_type: Some("object".to_string()),
        properties,
        required: vec!["name".to_string(), "count".to_string()],
        ..Default::default()
    };

    let columns = extract_columns(&schema, &spec, false);

    let name_col = columns.iter().find(|c| c.name == "name").unwrap();
    // name is required but nullable (from type array) → nullable=true
    assert!(name_col.nullable);
    assert_eq!(name_col.pg_type, "text");

    let count_col = columns.iter().find(|c| c.name == "count").unwrap();
    // count is required and not nullable → nullable=false
    assert!(!count_col.nullable);
    assert_eq!(count_col.pg_type, "bigint");
}

#[test]
fn test_rowid_selection_no_id_column() {
    // No 'id' column → picks first non-attrs non-jsonb column as rowid
    let spec =
        OpenApiSpec::from_str(r#"{"openapi": "3.0.0", "info": {"title": "T"}, "paths": {}}"#)
            .unwrap();

    let mut properties = HashMap::new();
    properties.insert(
        "name".to_string(),
        crate::spec::Schema {
            schema_type: Some("string".to_string()),
            ..Default::default()
        },
    );
    properties.insert(
        "metadata".to_string(),
        crate::spec::Schema {
            schema_type: Some("object".to_string()),
            ..Default::default()
        },
    );

    let schema = crate::spec::Schema {
        schema_type: Some("object".to_string()),
        properties,
        ..Default::default()
    };

    let endpoint = crate::spec::EndpointInfo {
        path: "/things".to_string(),
        method: "GET".to_string(),
        response_schema: Some(schema),
    };

    let table = generate_foreign_table(&endpoint, &spec, "test_server", false);
    // 'metadata' is jsonb, so 'name' (text) should be the rowid
    assert!(
        table.contains("rowid_column 'name'"),
        "Expected name as rowid: {table}"
    );
}

#[test]
fn test_rowid_selection_all_jsonb() {
    // All columns are jsonb → omits rowid_column
    let spec =
        OpenApiSpec::from_str(r#"{"openapi": "3.0.0", "info": {"title": "T"}, "paths": {}}"#)
            .unwrap();

    let mut properties = HashMap::new();
    properties.insert(
        "data".to_string(),
        crate::spec::Schema {
            schema_type: Some("object".to_string()),
            ..Default::default()
        },
    );
    properties.insert(
        "meta".to_string(),
        crate::spec::Schema {
            schema_type: Some("array".to_string()),
            ..Default::default()
        },
    );

    let schema = crate::spec::Schema {
        schema_type: Some("object".to_string()),
        properties,
        ..Default::default()
    };

    let endpoint = crate::spec::EndpointInfo {
        path: "/blobs".to_string(),
        method: "GET".to_string(),
        response_schema: Some(schema),
    };

    let table = generate_foreign_table(&endpoint, &spec, "test_server", false);
    // All columns are jsonb → no suitable rowid column
    assert!(
        !table.contains("rowid_column"),
        "All-jsonb schema should omit rowid_column: {table}"
    );
}

#[test]
fn test_no_properties_schema_defaults() {
    // Empty properties (e.g., additionalProperties-only schema) → only attrs column if enabled
    let spec =
        OpenApiSpec::from_str(r#"{"openapi": "3.0.0", "info": {"title": "T"}, "paths": {}}"#)
            .unwrap();

    let schema = crate::spec::Schema {
        schema_type: Some("object".to_string()),
        // No properties
        ..Default::default()
    };

    let columns_with_attrs = extract_columns(&schema, &spec, true);
    assert_eq!(columns_with_attrs.len(), 1);
    assert_eq!(columns_with_attrs[0].name, "attrs");

    let columns_without_attrs = extract_columns(&schema, &spec, false);
    assert_eq!(columns_without_attrs.len(), 0);
}

#[test]
fn test_column_ordering_id_first() {
    // id sorts first, rest alphabetical
    let spec =
        OpenApiSpec::from_str(r#"{"openapi": "3.0.0", "info": {"title": "T"}, "paths": {}}"#)
            .unwrap();

    let mut properties = HashMap::new();
    for name in &["zebra", "id", "alpha", "middle"] {
        properties.insert(
            name.to_string(),
            crate::spec::Schema {
                schema_type: Some("string".to_string()),
                ..Default::default()
            },
        );
    }

    let schema = crate::spec::Schema {
        schema_type: Some("object".to_string()),
        properties,
        ..Default::default()
    };

    let columns = extract_columns(&schema, &spec, false);
    let names: Vec<&str> = columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["id", "alpha", "middle", "zebra"]);
}

#[test]
fn test_sanitize_consecutive_special_chars() {
    // @@@id → ___id (each special char becomes _)
    assert_eq!(sanitize_column_name("@@@id"), "___id");
}

#[test]
fn test_sanitize_leading_underscore_preserved() {
    // _id stays _id (leading underscore preserved)
    assert_eq!(sanitize_column_name("_id"), "_id");
}
