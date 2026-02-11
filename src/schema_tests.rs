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
        "Expected user_name in {:?}",
        names
    );
    assert!(
        names.contains(&"user_name_1"),
        "Expected user_name_1 for collision in {:?}",
        names
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
