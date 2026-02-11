use super::*;

// --- json_to_rows tests ---

#[test]
fn test_json_to_rows_array() {
    let data = serde_json::json!([
        {"id": 1, "name": "alice"},
        {"id": 2, "name": "bob"},
        {"id": 3, "name": "charlie"}
    ]);
    let rows = OpenApiFdw::json_to_rows(data).unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0]["id"], 1);
    assert_eq!(rows[2]["name"], "charlie");
}

#[test]
fn test_json_to_rows_single_object() {
    let data = serde_json::json!({"id": 1, "name": "alice"});
    let rows = OpenApiFdw::json_to_rows(data).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["name"], "alice");
}

#[test]
fn test_json_to_rows_empty_array() {
    let data = serde_json::json!([]);
    let rows = OpenApiFdw::json_to_rows(data).unwrap();
    assert!(rows.is_empty());
}

#[test]
fn test_json_to_rows_rejects_primitive() {
    let data = serde_json::json!("just a string");
    assert!(OpenApiFdw::json_to_rows(data).is_err());
}

// --- extract_data tests ---

fn fdw_with_response_path(path: Option<&str>) -> OpenApiFdw {
    OpenApiFdw {
        response_path: path.map(String::from),
        ..Default::default()
    }
}

#[test]
fn test_extract_data_with_response_path() {
    let fdw = fdw_with_response_path(Some("/features"));
    let mut resp = serde_json::json!({
        "type": "FeatureCollection",
        "features": [
            {"properties": {"id": "a"}},
            {"properties": {"id": "b"}}
        ]
    });
    let rows = fdw.extract_data(&mut resp).unwrap();
    assert_eq!(rows.len(), 2);
    // Original is taken, not cloned
    assert!(resp["features"].is_null());
}

#[test]
fn test_extract_data_with_nested_response_path() {
    let fdw = fdw_with_response_path(Some("/result/data"));
    let mut resp = serde_json::json!({
        "result": {
            "data": [{"id": 1}, {"id": 2}, {"id": 3}]
        }
    });
    let rows = fdw.extract_data(&mut resp).unwrap();
    assert_eq!(rows.len(), 3);
}

#[test]
fn test_extract_data_invalid_response_path() {
    let fdw = fdw_with_response_path(Some("/nonexistent"));
    let mut resp = serde_json::json!({"data": [1, 2, 3]});
    assert!(fdw.extract_data(&mut resp).is_err());
}

#[test]
fn test_extract_data_direct_array() {
    let fdw = fdw_with_response_path(None);
    let mut resp = serde_json::json!([{"id": 1}, {"id": 2}]);
    let rows = fdw.extract_data(&mut resp).unwrap();
    assert_eq!(rows.len(), 2);
}

#[test]
fn test_extract_data_auto_detect_data_key() {
    let fdw = fdw_with_response_path(None);
    let mut resp = serde_json::json!({
        "data": [{"id": 1}, {"id": 2}],
        "meta": {"total": 2}
    });
    let rows = fdw.extract_data(&mut resp).unwrap();
    assert_eq!(rows.len(), 2);
    assert!(resp["data"].is_null());
}

#[test]
fn test_extract_data_auto_detect_results_key() {
    let fdw = fdw_with_response_path(None);
    let mut resp = serde_json::json!({
        "results": [{"id": "x"}],
        "count": 1
    });
    let rows = fdw.extract_data(&mut resp).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["id"], "x");
}

#[test]
fn test_extract_data_auto_detect_features_key() {
    let fdw = fdw_with_response_path(None);
    let mut resp = serde_json::json!({
        "type": "FeatureCollection",
        "features": [
            {"type": "Feature", "properties": {"name": "A"}},
            {"type": "Feature", "properties": {"name": "B"}}
        ]
    });
    let rows = fdw.extract_data(&mut resp).unwrap();
    assert_eq!(rows.len(), 2);
}

#[test]
fn test_extract_data_single_object_fallback() {
    let fdw = fdw_with_response_path(None);
    let mut resp = serde_json::json!({
        "id": "abc",
        "name": "singleton"
    });
    let rows = fdw.extract_data(&mut resp).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["id"], "abc");
}

#[test]
fn test_extract_data_ownership_no_clone() {
    // Verify that extract_data takes ownership rather than cloning:
    // after extraction, the original data should be replaced with null
    let fdw = fdw_with_response_path(Some("/items"));
    let mut resp = serde_json::json!({
        "items": [
            {"id": 1, "payload": "x".repeat(1000)},
            {"id": 2, "payload": "y".repeat(1000)}
        ]
    });
    let rows = fdw.extract_data(&mut resp).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["payload"].as_str().unwrap().len(), 1000);
    // The original value was taken, not cloned
    assert!(resp.pointer("/items").unwrap().is_null());
}

// --- to_camel_case tests ---

#[test]
fn test_to_camel_case() {
    assert_eq!(to_camel_case("snake_case"), "snakeCase");
    assert_eq!(to_camel_case("already"), "already");
    assert_eq!(to_camel_case("multi_word_name"), "multiWordName");
    assert_eq!(to_camel_case(""), "");
}

// --- build_column_key_map tests ---

/// Helper: create an FDW with cached columns and src_rows, then build key map
fn build_key_map(
    col_names: &[&str],
    rows: Vec<JsonValue>,
    object_path: Option<&str>,
) -> Vec<Option<KeyMatch>> {
    let mut fdw = OpenApiFdw {
        src_rows: rows,
        object_path: object_path.map(String::from),
        ..Default::default()
    };
    fdw.cached_columns = col_names
        .iter()
        .map(|name| CachedColumn {
            name: name.to_string(),
            type_oid: TypeOid::String,
            camel_name: to_camel_case(name),
            lower_name: name.to_lowercase(),
            alnum_name: name
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
                .to_lowercase(),
        })
        .collect();
    fdw.build_column_key_map();
    fdw.column_key_map
}

#[test]
fn test_build_column_key_map_exact() {
    let rows = vec![serde_json::json!({"id": 1, "name": "alice"})];
    let map = build_key_map(&["id", "name"], rows, None);
    assert_eq!(map, vec![Some(KeyMatch::Exact), Some(KeyMatch::Exact)]);
}

#[test]
fn test_build_column_key_map_camel() {
    // API returns camelCase, SQL columns are snake_case
    let rows = vec![serde_json::json!({"firstName": "Alice", "lastName": "Smith"})];
    let map = build_key_map(&["first_name", "last_name"], rows, None);
    assert_eq!(
        map,
        vec![Some(KeyMatch::CamelCase), Some(KeyMatch::CamelCase)]
    );
}

#[test]
fn test_build_column_key_map_case_insensitive() {
    // API returns PascalCase, SQL columns are lowercase
    let rows = vec![serde_json::json!({"Id": 1, "UserName": "alice"})];
    let map = build_key_map(&["id", "username"], rows, None);
    assert_eq!(
        map,
        vec![
            Some(KeyMatch::CaseInsensitive("Id".to_string())),
            Some(KeyMatch::CaseInsensitive("UserName".to_string()))
        ]
    );
}

#[test]
fn test_build_column_key_map_empty_rows() {
    let map = build_key_map(&["id", "name"], vec![], None);
    assert_eq!(map, vec![None, None]);
}

#[test]
fn test_build_column_key_map_missing_column() {
    let rows = vec![serde_json::json!({"id": 1, "name": "alice"})];
    let map = build_key_map(&["id", "email"], rows, None);
    assert_eq!(map, vec![Some(KeyMatch::Exact), None]);
}

#[test]
fn test_build_column_key_map_attrs_skipped() {
    let rows = vec![serde_json::json!({"id": 1, "name": "alice"})];
    let map = build_key_map(&["id", "attrs"], rows, None);
    // attrs should be None (special-cased, not looked up)
    assert_eq!(map, vec![Some(KeyMatch::Exact), None]);
}

#[test]
fn test_build_column_key_map_with_object_path() {
    // GeoJSON-style: keys live under /properties
    let rows = vec![serde_json::json!({
        "type": "Feature",
        "properties": {"name": "Park", "area": 500}
    })];
    let map = build_key_map(&["name", "area"], rows, Some("/properties"));
    assert_eq!(map, vec![Some(KeyMatch::Exact), Some(KeyMatch::Exact)]);
}

// --- Pagination tests ---

fn make_fdw_for_pagination(cursor_path: &str) -> OpenApiFdw {
    OpenApiFdw {
        cursor_path: cursor_path.to_string(),
        cursor_param: "after".to_string(),
        ..Default::default()
    }
}

#[test]
fn test_handle_pagination_cursor_path_token() {
    let mut fdw = make_fdw_for_pagination("/cursor");
    let resp = serde_json::json!({"cursor": "abc123", "data": []});
    fdw.handle_pagination(&resp);
    assert_eq!(fdw.next_cursor, Some("abc123".to_string()));
    assert_eq!(fdw.next_url, None);
}

#[test]
fn test_handle_pagination_cursor_path_full_url() {
    let mut fdw = make_fdw_for_pagination("/pagination/next");
    let resp = serde_json::json!({
        "pagination": {"next": "https://api.example.com/items?cursor=xyz"},
        "data": []
    });
    fdw.handle_pagination(&resp);
    assert_eq!(fdw.next_cursor, None);
    assert_eq!(
        fdw.next_url,
        Some("https://api.example.com/items?cursor=xyz".to_string())
    );
}

#[test]
fn test_handle_pagination_cursor_path_http_url() {
    let mut fdw = make_fdw_for_pagination("/next");
    let resp = serde_json::json!({"next": "http://api.example.com/page2"});
    fdw.handle_pagination(&resp);
    assert_eq!(fdw.next_cursor, None);
    assert_eq!(
        fdw.next_url,
        Some("http://api.example.com/page2".to_string())
    );
}

#[test]
fn test_handle_pagination_cursor_path_missing() {
    let mut fdw = make_fdw_for_pagination("/cursor");
    let resp = serde_json::json!({"data": []});
    fdw.handle_pagination(&resp);
    assert_eq!(fdw.next_cursor, None);
    assert_eq!(fdw.next_url, None);
}

#[test]
fn test_handle_pagination_auto_detect_next_url() {
    let mut fdw = make_fdw_for_pagination(""); // no cursor_path configured
    let resp = serde_json::json!({
        "pagination": {"next": "https://api.example.com/items?page=2"},
        "data": []
    });
    fdw.handle_pagination(&resp);
    assert_eq!(fdw.next_cursor, None);
    assert_eq!(
        fdw.next_url,
        Some("https://api.example.com/items?page=2".to_string())
    );
}

// --- Pagination auto-detection edge cases ---

#[test]
fn test_handle_pagination_auto_detect_links_next() {
    // HAL-style: /links/next
    let mut fdw = make_fdw_for_pagination("");
    let resp = serde_json::json!({
        "links": {"next": "https://api.example.com/page2"},
        "data": []
    });
    fdw.handle_pagination(&resp);
    assert_eq!(
        fdw.next_url,
        Some("https://api.example.com/page2".to_string())
    );
}

#[test]
fn test_handle_pagination_auto_detect_has_more_with_cursor() {
    // Stripe-style: has_more + next_cursor
    let mut fdw = make_fdw_for_pagination("");
    let resp = serde_json::json!({
        "has_more": true,
        "next_cursor": "cursor_xyz",
        "data": [{"id": 1}]
    });
    fdw.handle_pagination(&resp);
    assert_eq!(fdw.next_cursor, Some("cursor_xyz".to_string()));
    assert_eq!(fdw.next_url, None);
}

#[test]
fn test_handle_pagination_has_more_false_stops() {
    // has_more: false should NOT set any pagination
    let mut fdw = make_fdw_for_pagination("");
    let resp = serde_json::json!({
        "has_more": false,
        "next_cursor": "stale_cursor",
        "data": [{"id": 1}]
    });
    fdw.handle_pagination(&resp);
    assert_eq!(fdw.next_cursor, None);
    assert_eq!(fdw.next_url, None);
}

#[test]
fn test_handle_pagination_auto_detect_meta_pagination() {
    // Nested meta.pagination.next pattern
    let mut fdw = make_fdw_for_pagination("");
    let resp = serde_json::json!({
        "meta": {
            "pagination": {
                "next": "https://api.example.com/items?page=3"
            }
        },
        "data": []
    });
    fdw.handle_pagination(&resp);
    assert_eq!(
        fdw.next_url,
        Some("https://api.example.com/items?page=3".to_string())
    );
}

#[test]
fn test_handle_pagination_empty_string_next_url_stops() {
    // Empty string cursor_path value should be treated as "no more pages"
    let mut fdw = make_fdw_for_pagination("/next");
    let resp = serde_json::json!({"next": "", "data": []});
    fdw.handle_pagination(&resp);
    assert_eq!(fdw.next_cursor, None);
    assert_eq!(fdw.next_url, None);
}

#[test]
fn test_handle_pagination_null_cursor_stops() {
    // Null cursor should mean end of pagination
    let mut fdw = make_fdw_for_pagination("/cursor");
    let resp = serde_json::json!({"cursor": null, "data": []});
    fdw.handle_pagination(&resp);
    assert_eq!(fdw.next_cursor, None);
    assert_eq!(fdw.next_url, None);
}

#[test]
fn test_handle_pagination_array_response_no_autodetect() {
    // Auto-detection should not run on array responses
    let mut fdw = make_fdw_for_pagination("");
    let resp = serde_json::json!([{"id": 1}, {"id": 2}]);
    fdw.handle_pagination(&resp);
    assert_eq!(fdw.next_cursor, None);
    assert_eq!(fdw.next_url, None);
}

// --- Column key map edge cases ---

#[test]
fn test_build_column_key_map_at_prefixed_keys() {
    // JSON-LD @-prefixed keys: @id sanitizes to _id.
    // The normalized matching step strips non-alnum chars:
    //   column "_id" → alnum "id", key "@id" → alnum "id" → match!
    let rows = vec![serde_json::json!({"@id": "urn:test", "@type": "Feature"})];
    let map = build_key_map(&["_id", "_type"], rows, None);
    assert_eq!(
        map,
        vec![
            Some(KeyMatch::CaseInsensitive("@id".to_string())),
            Some(KeyMatch::CaseInsensitive("@type".to_string()))
        ]
    );
}

#[test]
fn test_build_column_key_map_dotted_keys() {
    // Dotted property names: "user.name" sanitizes to "user_name"
    // Normalized: "username" == "username" → match
    let rows = vec![serde_json::json!({"user.name": "Alice", "user.email": "a@b.com"})];
    let map = build_key_map(&["user_name", "user_email"], rows, None);
    assert_eq!(
        map,
        vec![
            Some(KeyMatch::CaseInsensitive("user.name".to_string())),
            Some(KeyMatch::CaseInsensitive("user.email".to_string()))
        ]
    );
}

#[test]
fn test_build_column_key_map_dollar_prefixed_keys() {
    // MongoDB-style $-prefixed keys: "$oid" sanitizes to "_oid"
    let rows = vec![serde_json::json!({"$oid": "abc123"})];
    let map = build_key_map(&["_oid"], rows, None);
    assert_eq!(
        map,
        vec![Some(KeyMatch::CaseInsensitive("$oid".to_string()))]
    );
}

#[test]
fn test_build_column_key_map_mixed_conventions() {
    // Mixed API response: some exact, some camel, some case-insensitive
    // Note: case-insensitive compares k.to_lowercase() == col.lower_name,
    // so it only works for pure case differences (e.g., "Status" vs "status"),
    // not camelCase→snake_case transformations.
    let rows = vec![serde_json::json!({
        "id": 1,
        "firstName": "Alice",
        "Status": "active"
    })];
    let map = build_key_map(&["id", "first_name", "status"], rows, None);
    assert_eq!(
        map,
        vec![
            Some(KeyMatch::Exact),
            Some(KeyMatch::CamelCase),
            Some(KeyMatch::CaseInsensitive("Status".to_string()))
        ]
    );
}

// --- extract_data edge cases ---

#[test]
fn test_extract_data_auto_detect_records_key() {
    let fdw = fdw_with_response_path(None);
    let mut resp = serde_json::json!({
        "records": [{"id": 1}, {"id": 2}],
        "total": 2
    });
    let rows = fdw.extract_data(&mut resp).unwrap();
    assert_eq!(rows.len(), 2);
}

#[test]
fn test_extract_data_auto_detect_entries_key() {
    let fdw = fdw_with_response_path(None);
    let mut resp = serde_json::json!({
        "entries": [{"id": "a"}, {"id": "b"}]
    });
    let rows = fdw.extract_data(&mut resp).unwrap();
    assert_eq!(rows.len(), 2);
}

#[test]
fn test_extract_data_auto_detect_items_key() {
    let fdw = fdw_with_response_path(None);
    let mut resp = serde_json::json!({
        "items": [{"id": 1}],
        "next_page": null
    });
    let rows = fdw.extract_data(&mut resp).unwrap();
    assert_eq!(rows.len(), 1);
}

#[test]
fn test_extract_data_priority_order() {
    // When response has both "data" and "results", "data" wins (checked first)
    let fdw = fdw_with_response_path(None);
    let mut resp = serde_json::json!({
        "data": [{"id": 1}],
        "results": [{"id": 2}, {"id": 3}]
    });
    let rows = fdw.extract_data(&mut resp).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["id"], 1);
}

#[test]
fn test_extract_data_non_array_wrapper_becomes_single_row() {
    // If a wrapper key contains an object (not array), treat as single row
    let fdw = fdw_with_response_path(None);
    let mut resp = serde_json::json!({
        "data": {"id": "single", "name": "test"}
    });
    let rows = fdw.extract_data(&mut resp).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["id"], "single");
}

#[test]
fn test_extract_data_auto_detect_at_graph_key() {
    // JSON-LD @graph wrapper (NWS API with Accept: application/ld+json)
    let fdw = fdw_with_response_path(None);
    let mut resp = serde_json::json!({
        "@context": {"@version": "1.1", "wx": "https://api.weather.gov/ontology#"},
        "@graph": [
            {"@id": "urn:alert:1", "@type": "wx:Alert", "headline": "Storm warning"},
            {"@id": "urn:alert:2", "@type": "wx:Alert", "headline": "Heat advisory"}
        ]
    });
    let rows = fdw.extract_data(&mut resp).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["headline"], "Storm warning");
}

// --- normalize_datetime tests ---

#[test]
fn test_normalize_datetime_date_only() {
    assert_eq!(
        OpenApiFdw::normalize_datetime("2024-01-15"),
        "2024-01-15T00:00:00Z"
    );
}

#[test]
fn test_normalize_datetime_full_datetime() {
    // Full datetime should pass through unchanged
    let dt = "2024-06-15T10:30:00Z";
    assert_eq!(OpenApiFdw::normalize_datetime(dt), dt);
}

#[test]
fn test_normalize_datetime_with_offset() {
    let dt = "2024-06-15T10:30:00+05:00";
    assert_eq!(OpenApiFdw::normalize_datetime(dt), dt);
}

#[test]
fn test_normalize_datetime_not_date_format() {
    // Strings that are 10 chars but not date format should pass through
    assert_eq!(OpenApiFdw::normalize_datetime("abcdefghij"), "abcdefghij");
}

// --- to_camel_case edge cases ---

#[test]
fn test_to_camel_case_trailing_underscore() {
    assert_eq!(to_camel_case("name_"), "name");
}

#[test]
fn test_to_camel_case_double_underscore() {
    assert_eq!(to_camel_case("a__b"), "aB");
}

#[test]
fn test_to_camel_case_single_char() {
    assert_eq!(to_camel_case("x"), "x");
}

// --- resolve_pagination_url tests ---

fn make_fdw_for_url(base_url: &str, endpoint: &str) -> OpenApiFdw {
    OpenApiFdw {
        base_url: base_url.to_string(),
        endpoint: endpoint.to_string(),
        ..Default::default()
    }
}

#[test]
fn test_resolve_pagination_url_absolute_https() {
    let fdw = make_fdw_for_url("https://api.example.com", "/items");
    let url = fdw.resolve_pagination_url("https://api.example.com/items?page=2&limit=10");
    assert_eq!(url, "https://api.example.com/items?page=2&limit=10");
}

#[test]
fn test_resolve_pagination_url_absolute_http() {
    let fdw = make_fdw_for_url("http://mockserver:1080", "/items");
    let url = fdw.resolve_pagination_url("http://mockserver:1080/items?page=2");
    assert_eq!(url, "http://mockserver:1080/items?page=2");
}

#[test]
fn test_resolve_pagination_url_query_only() {
    // "?page=2" should resolve against base_url + endpoint
    let fdw = make_fdw_for_url("https://api.example.com", "/items");
    let url = fdw.resolve_pagination_url("?page=2");
    assert_eq!(url, "https://api.example.com/items?page=2");
}

#[test]
fn test_resolve_pagination_url_query_only_strips_existing_query() {
    // If endpoint already has query params, only the path part is used
    let fdw = make_fdw_for_url("https://api.example.com", "/items?status=active");
    let url = fdw.resolve_pagination_url("?page=2");
    assert_eq!(url, "https://api.example.com/items?page=2");
}

#[test]
fn test_resolve_pagination_url_absolute_path() {
    // "/items?page=2" should resolve against base_url
    let fdw = make_fdw_for_url("https://api.example.com", "/old-endpoint");
    let url = fdw.resolve_pagination_url("/items?page=2&limit=50");
    assert_eq!(url, "https://api.example.com/items?page=2&limit=50");
}

#[test]
fn test_resolve_pagination_url_bare_relative() {
    // "page/2" should resolve against base_url/
    let fdw = make_fdw_for_url("https://api.example.com", "/items");
    let url = fdw.resolve_pagination_url("page/2");
    assert_eq!(url, "https://api.example.com/page/2");
}

// --- json_to_cell_cached edge cases ---

/// Helper: build an FDW with cached columns and column_key_map, then call json_to_cell_cached
fn cell_from_json(
    col_name: &str,
    type_oid: TypeOid,
    json_obj: &JsonValue,
) -> Result<Option<Cell>, String> {
    let mut fdw = OpenApiFdw::default();
    fdw.cached_columns = vec![CachedColumn {
        name: col_name.to_string(),
        type_oid: type_oid.clone(),
        camel_name: to_camel_case(col_name),
        lower_name: col_name.to_lowercase(),
        alnum_name: col_name
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
            .to_lowercase(),
    }];
    // Pre-populate key map with exact match
    fdw.column_key_map = vec![Some(KeyMatch::Exact)];
    fdw.json_to_cell_cached(json_obj, 0)
}

/// Helper: extract string from Cell
fn cell_to_string(cell: &Cell) -> Option<String> {
    match cell {
        Cell::String(s) => Some(s.clone()),
        _ => None,
    }
}

#[test]
fn test_json_to_cell_bool() {
    let obj = serde_json::json!({"active": true});
    let cell = cell_from_json("active", TypeOid::Bool, &obj).unwrap();
    assert!(matches!(cell, Some(Cell::Bool(true))));
}

#[test]
fn test_json_to_cell_i8() {
    let obj = serde_json::json!({"val": 42});
    let cell = cell_from_json("val", TypeOid::I8, &obj).unwrap();
    assert!(matches!(cell, Some(Cell::I8(42))));
}

#[test]
fn test_json_to_cell_i8_overflow() {
    // 200 exceeds i8 range (-128..127)
    let obj = serde_json::json!({"val": 200});
    let cell = cell_from_json("val", TypeOid::I8, &obj).unwrap();
    assert!(cell.is_none());
}

#[test]
fn test_json_to_cell_i16() {
    let obj = serde_json::json!({"val": 1000});
    let cell = cell_from_json("val", TypeOid::I16, &obj).unwrap();
    assert!(matches!(cell, Some(Cell::I16(1000))));
}

#[test]
fn test_json_to_cell_i32() {
    let obj = serde_json::json!({"val": 100_000});
    let cell = cell_from_json("val", TypeOid::I32, &obj).unwrap();
    assert!(matches!(cell, Some(Cell::I32(100_000))));
}

#[test]
fn test_json_to_cell_i64() {
    let obj = serde_json::json!({"val": 9_000_000_000_i64});
    let cell = cell_from_json("val", TypeOid::I64, &obj).unwrap();
    assert!(matches!(cell, Some(Cell::I64(9_000_000_000))));
}

#[test]
fn test_json_to_cell_f32() {
    let obj = serde_json::json!({"val": 3.14});
    let cell = cell_from_json("val", TypeOid::F32, &obj).unwrap();
    if let Some(Cell::F32(v)) = cell {
        assert!((v - 3.14_f32).abs() < 0.001);
    } else {
        panic!("Expected F32");
    }
}

#[test]
fn test_json_to_cell_f64() {
    let obj = serde_json::json!({"val": 3.141_592_653_589_793});
    let cell = cell_from_json("val", TypeOid::F64, &obj).unwrap();
    if let Some(Cell::F64(v)) = cell {
        assert!((v - std::f64::consts::PI).abs() < f64::EPSILON);
    } else {
        panic!("Expected F64");
    }
}

#[test]
fn test_json_to_cell_numeric() {
    let obj = serde_json::json!({"val": 99.99});
    let cell = cell_from_json("val", TypeOid::Numeric, &obj).unwrap();
    if let Some(Cell::Numeric(v)) = cell {
        assert!((v - 99.99).abs() < f64::EPSILON);
    } else {
        panic!("Expected Numeric");
    }
}

#[test]
fn test_json_to_cell_string_from_string() {
    let obj = serde_json::json!({"name": "alice"});
    let cell = cell_from_json("name", TypeOid::String, &obj).unwrap();
    assert_eq!(
        cell_to_string(cell.as_ref().unwrap()),
        Some("alice".to_string())
    );
}

#[test]
fn test_json_to_cell_string_from_number() {
    // Non-string values should be serialized to string
    let obj = serde_json::json!({"name": 42});
    let cell = cell_from_json("name", TypeOid::String, &obj).unwrap();
    assert_eq!(
        cell_to_string(cell.as_ref().unwrap()),
        Some("42".to_string())
    );
}

#[test]
fn test_json_to_cell_string_from_bool() {
    let obj = serde_json::json!({"name": true});
    let cell = cell_from_json("name", TypeOid::String, &obj).unwrap();
    assert_eq!(
        cell_to_string(cell.as_ref().unwrap()),
        Some("true".to_string())
    );
}

#[test]
fn test_json_to_cell_null_returns_none() {
    let obj = serde_json::json!({"val": null});
    let cell = cell_from_json("val", TypeOid::String, &obj).unwrap();
    assert!(cell.is_none());
}

#[test]
fn test_json_to_cell_missing_key_returns_none() {
    let obj = serde_json::json!({"other": "value"});
    let mut fdw = OpenApiFdw::default();
    fdw.cached_columns = vec![CachedColumn {
        name: "missing".to_string(),
        type_oid: TypeOid::String,
        camel_name: "missing".to_string(),
        lower_name: "missing".to_string(),
        alnum_name: "missing".to_string(),
    }];
    fdw.column_key_map = vec![None]; // no match found
    let cell = fdw.json_to_cell_cached(&obj, 0).unwrap();
    assert!(cell.is_none());
}

// Note: Date/Timestamp/Timestamptz from string tests are skipped because
// they call `time::parse_from_rfc3339` which is a WASM host import that
// panics outside the WASM runtime. These paths are covered by integration tests.

#[test]
fn test_json_to_cell_date_from_unix() {
    // Unix timestamp as integer → Date (doesn't call parse_from_rfc3339)
    let obj = serde_json::json!({"dt": 1718409600});
    let cell = cell_from_json("dt", TypeOid::Date, &obj).unwrap();
    assert!(matches!(cell, Some(Cell::Date(1718409600))));
}

#[test]
fn test_json_to_cell_timestamp_from_unix() {
    // Unix epoch → microseconds (doesn't call parse_from_rfc3339)
    let obj = serde_json::json!({"ts": 1718409600});
    let cell = cell_from_json("ts", TypeOid::Timestamp, &obj).unwrap();
    if let Some(Cell::Timestamp(v)) = cell {
        assert_eq!(v, 1_718_409_600_000_000);
    } else {
        panic!("Expected Timestamp");
    }
}

#[test]
fn test_json_to_cell_timestamptz_from_unix() {
    // Unix epoch → microseconds (doesn't call parse_from_rfc3339)
    let obj = serde_json::json!({"ts": 1718409600});
    let cell = cell_from_json("ts", TypeOid::Timestamptz, &obj).unwrap();
    if let Some(Cell::Timestamptz(v)) = cell {
        assert_eq!(v, 1_718_409_600_000_000);
    } else {
        panic!("Expected Timestamptz");
    }
}

#[test]
fn test_json_to_cell_uuid() {
    let obj = serde_json::json!({"uid": "550e8400-e29b-41d4-a716-446655440000"});
    let cell = cell_from_json("uid", TypeOid::Uuid, &obj).unwrap();
    assert_eq!(
        cell_to_string(cell.as_ref().unwrap()),
        Some("550e8400-e29b-41d4-a716-446655440000".to_string())
    );
}

#[test]
fn test_json_to_cell_json_object() {
    let obj = serde_json::json!({"meta": {"key": "val"}});
    let cell = cell_from_json("meta", TypeOid::Json, &obj).unwrap();
    if let Some(Cell::Json(s)) = cell {
        assert_eq!(s, r#"{"key":"val"}"#);
    } else {
        panic!("Expected Json");
    }
}

#[test]
fn test_json_to_cell_json_array() {
    let obj = serde_json::json!({"tags": ["a", "b", "c"]});
    let cell = cell_from_json("tags", TypeOid::Json, &obj).unwrap();
    if let Some(Cell::Json(s)) = cell {
        assert_eq!(s, r#"["a","b","c"]"#);
    } else {
        panic!("Expected Json");
    }
}

#[test]
fn test_json_to_cell_path_param_injection() {
    // When a column is a path param, its value is injected from path_params
    let mut fdw = OpenApiFdw::default();
    fdw.cached_columns = vec![CachedColumn {
        name: "user_id".to_string(),
        type_oid: TypeOid::String,
        camel_name: "userId".to_string(),
        lower_name: "user_id".to_string(),
        alnum_name: "userid".to_string(),
    }];
    fdw.column_key_map = vec![None];
    fdw.path_params
        .insert("user_id".to_string(), "42".to_string());

    let obj = serde_json::json!({"title": "Post Title"});
    let cell = fdw.json_to_cell_cached(&obj, 0).unwrap();
    assert_eq!(
        cell_to_string(cell.as_ref().unwrap()),
        Some("42".to_string())
    );
}

#[test]
fn test_json_to_cell_path_param_type_coercion() {
    // Path param for an integer column should be coerced
    let mut fdw = OpenApiFdw::default();
    fdw.cached_columns = vec![CachedColumn {
        name: "id".to_string(),
        type_oid: TypeOid::I64,
        camel_name: "id".to_string(),
        lower_name: "id".to_string(),
        alnum_name: "id".to_string(),
    }];
    fdw.column_key_map = vec![None];
    fdw.path_params.insert("id".to_string(), "123".to_string());

    let obj = serde_json::json!({});
    let cell = fdw.json_to_cell_cached(&obj, 0).unwrap();
    assert!(matches!(cell, Some(Cell::I64(123))));
}

#[test]
fn test_json_to_cell_attrs_column() {
    // The "attrs" column gets the full row as JSON
    let mut fdw = OpenApiFdw::default();
    fdw.cached_columns = vec![CachedColumn {
        name: "attrs".to_string(),
        type_oid: TypeOid::Json,
        camel_name: "attrs".to_string(),
        lower_name: "attrs".to_string(),
        alnum_name: "attrs".to_string(),
    }];
    fdw.column_key_map = vec![None]; // attrs is special-cased, no key match

    let obj = serde_json::json!({"id": 1, "name": "test"});
    let cell = fdw.json_to_cell_cached(&obj, 0).unwrap();
    assert!(matches!(cell, Some(Cell::Json(_))));
}

#[test]
fn test_json_to_cell_fallback_camel_match() {
    // When column_key_map has None (heterogeneous rows), fallback to camelCase match
    let mut fdw = OpenApiFdw::default();
    fdw.cached_columns = vec![CachedColumn {
        name: "first_name".to_string(),
        type_oid: TypeOid::String,
        camel_name: "firstName".to_string(),
        lower_name: "first_name".to_string(),
        alnum_name: "firstname".to_string(),
    }];
    fdw.column_key_map = vec![None]; // force fallback path

    let obj = serde_json::json!({"firstName": "Alice"});
    let cell = fdw.json_to_cell_cached(&obj, 0).unwrap();
    assert_eq!(
        cell_to_string(cell.as_ref().unwrap()),
        Some("Alice".to_string())
    );
}

#[test]
fn test_json_to_cell_fallback_normalized_match() {
    // Fallback to normalized (alnum-only) matching for @-prefixed keys
    let mut fdw = OpenApiFdw::default();
    fdw.cached_columns = vec![CachedColumn {
        name: "_id".to_string(),
        type_oid: TypeOid::String,
        camel_name: "Id".to_string(),
        lower_name: "_id".to_string(),
        alnum_name: "id".to_string(),
    }];
    fdw.column_key_map = vec![None]; // force fallback path

    let obj = serde_json::json!({"@id": "urn:test:123"});
    let cell = fdw.json_to_cell_cached(&obj, 0).unwrap();
    assert_eq!(
        cell_to_string(cell.as_ref().unwrap()),
        Some("urn:test:123".to_string())
    );
}
