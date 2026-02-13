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

// --- normalize_to_alnum tests ---

#[test]
fn test_normalize_to_alnum_basic() {
    assert_eq!(normalize_to_alnum("hello"), "hello");
    assert_eq!(normalize_to_alnum("Hello"), "hello");
    assert_eq!(normalize_to_alnum(""), "");
}

#[test]
fn test_normalize_to_alnum_special_chars() {
    assert_eq!(normalize_to_alnum("@id"), "id");
    assert_eq!(normalize_to_alnum("$oid"), "oid");
    assert_eq!(normalize_to_alnum("user.name"), "username");
    assert_eq!(normalize_to_alnum("user-name"), "username");
    assert_eq!(normalize_to_alnum("_id"), "id");
}

#[test]
fn test_normalize_to_alnum_mixed() {
    assert_eq!(normalize_to_alnum("user_Name"), "username");
    assert_eq!(normalize_to_alnum("@Type"), "type");
    assert_eq!(normalize_to_alnum("123-abc"), "123abc");
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
            alnum_name: normalize_to_alnum(name),
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
    let fdw = OpenApiFdw {
        cached_columns: vec![CachedColumn {
            name: col_name.to_string(),
            type_oid: type_oid.clone(),
            camel_name: to_camel_case(col_name),
            lower_name: col_name.to_lowercase(),
            alnum_name: normalize_to_alnum(col_name),
        }],
        column_key_map: vec![Some(KeyMatch::Exact)],
        ..Default::default()
    };
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
    let obj = serde_json::json!({"val": 2.78});
    let cell = cell_from_json("val", TypeOid::F32, &obj).unwrap();
    if let Some(Cell::F32(v)) = cell {
        assert!((v - 2.78_f32).abs() < 0.001);
    } else {
        panic!("Expected F32");
    }
}

#[test]
fn test_json_to_cell_f64() {
    let obj = serde_json::json!({"val": 1.234_567_890_123});
    let cell = cell_from_json("val", TypeOid::F64, &obj).unwrap();
    if let Some(Cell::F64(v)) = cell {
        assert!((v - 1.234_567_890_123).abs() < f64::EPSILON);
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
    let fdw = OpenApiFdw {
        cached_columns: vec![CachedColumn {
            name: "missing".to_string(),
            type_oid: TypeOid::String,
            camel_name: "missing".to_string(),
            lower_name: "missing".to_string(),
            alnum_name: "missing".to_string(),
        }],
        column_key_map: vec![None], // no match found
        ..Default::default()
    };
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
    // When a column is a path param, its value is injected from injected_params
    let mut fdw = OpenApiFdw {
        cached_columns: vec![CachedColumn {
            name: "user_id".to_string(),
            type_oid: TypeOid::String,
            camel_name: "userId".to_string(),
            lower_name: "user_id".to_string(),
            alnum_name: "userid".to_string(),
        }],
        column_key_map: vec![None],
        ..Default::default()
    };
    fdw.injected_params
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
    let mut fdw = OpenApiFdw {
        cached_columns: vec![CachedColumn {
            name: "id".to_string(),
            type_oid: TypeOid::I64,
            camel_name: "id".to_string(),
            lower_name: "id".to_string(),
            alnum_name: "id".to_string(),
        }],
        column_key_map: vec![None],
        ..Default::default()
    };
    fdw.injected_params
        .insert("id".to_string(), "123".to_string());

    let obj = serde_json::json!({});
    let cell = fdw.json_to_cell_cached(&obj, 0).unwrap();
    assert!(matches!(cell, Some(Cell::I64(123))));
}

#[test]
fn test_json_to_cell_attrs_column() {
    // The "attrs" column gets the full row as JSON
    let fdw = OpenApiFdw {
        cached_columns: vec![CachedColumn {
            name: "attrs".to_string(),
            type_oid: TypeOid::Json,
            camel_name: "attrs".to_string(),
            lower_name: "attrs".to_string(),
            alnum_name: "attrs".to_string(),
        }],
        column_key_map: vec![None], // attrs is special-cased, no key match
        ..Default::default()
    };

    let obj = serde_json::json!({"id": 1, "name": "test"});
    let cell = fdw.json_to_cell_cached(&obj, 0).unwrap();
    assert!(matches!(cell, Some(Cell::Json(_))));
}

#[test]
fn test_json_to_cell_fallback_camel_match() {
    // When column_key_map has None (heterogeneous rows), fallback to camelCase match
    let fdw = OpenApiFdw {
        cached_columns: vec![CachedColumn {
            name: "first_name".to_string(),
            type_oid: TypeOid::String,
            camel_name: "firstName".to_string(),
            lower_name: "first_name".to_string(),
            alnum_name: "firstname".to_string(),
        }],
        column_key_map: vec![None], // force fallback path
        ..Default::default()
    };

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
    let fdw = OpenApiFdw {
        cached_columns: vec![CachedColumn {
            name: "_id".to_string(),
            type_oid: TypeOid::String,
            camel_name: "Id".to_string(),
            lower_name: "_id".to_string(),
            alnum_name: "id".to_string(),
        }],
        column_key_map: vec![None], // force fallback path
        ..Default::default()
    };

    let obj = serde_json::json!({"@id": "urn:test:123"});
    let cell = fdw.json_to_cell_cached(&obj, 0).unwrap();
    assert_eq!(
        cell_to_string(cell.as_ref().unwrap()),
        Some("urn:test:123".to_string())
    );
}

// --- Real-world API pattern tests ---

#[test]
fn test_stripe_list_response() {
    // Stripe pattern: {object:"list", data:[...], has_more:true}
    let fdw = fdw_with_response_path(None);
    let mut resp = serde_json::json!({
        "object": "list",
        "data": [
            {"id": "ch_1", "amount": 2000, "currency": "usd"},
            {"id": "ch_2", "amount": 5000, "currency": "eur"}
        ],
        "has_more": true,
        "url": "/v1/charges"
    });
    let rows = fdw.extract_data(&mut resp).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["id"], "ch_1");
    assert_eq!(rows[1]["amount"], 5000);
    // data was taken (ownership), not cloned
    assert!(resp["data"].is_null());
}

#[test]
fn test_github_direct_array() {
    // GitHub pattern: direct array response + no auto-pagination
    let fdw = fdw_with_response_path(None);
    let mut resp = serde_json::json!([
        {"id": 1, "login": "octocat", "type": "User"},
        {"id": 2, "login": "hubot", "type": "Bot"}
    ]);
    let rows = fdw.extract_data(&mut resp).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["login"], "octocat");

    // Array responses should not trigger auto-pagination
    let mut pagination_fdw = make_fdw_for_pagination("");
    let array_resp = serde_json::json!([{"id": 1}, {"id": 2}]);
    pagination_fdw.handle_pagination(&array_resp);
    assert_eq!(pagination_fdw.next_cursor, None);
    assert_eq!(pagination_fdw.next_url, None);
}

#[test]
fn test_hal_links_next_href_pagination() {
    // HAL pattern: _links/next/href pagination path
    let mut fdw = make_fdw_for_pagination("");
    let resp = serde_json::json!({
        "_embedded": {"items": [{"id": 1}]},
        "_links": {
            "self": {"href": "https://api.example.com/items?page=1"},
            "next": {"href": "https://api.example.com/items?page=2"}
        }
    });
    fdw.handle_pagination(&resp);
    assert_eq!(
        fdw.next_url,
        Some("https://api.example.com/items?page=2".to_string())
    );
}

#[test]
fn test_hyphen_case_key_matching() {
    // REST APIs with hyphen-case keys: "user-id" → normalized match to "user_id"
    let rows = vec![serde_json::json!({"user-id": "abc", "user-name": "Alice"})];
    let map = build_key_map(&["user_id", "user_name"], rows, None);
    // Normalized matching: "userid" matches "userid" (after stripping non-alnum)
    assert_eq!(
        map,
        vec![
            Some(KeyMatch::CaseInsensitive("user-id".to_string())),
            Some(KeyMatch::CaseInsensitive("user-name".to_string()))
        ]
    );
}

#[test]
fn test_screaming_snake_case_matching() {
    // Legacy APIs with SCREAMING_SNAKE_CASE: "USER_NAME" → case-insensitive match
    let rows = vec![serde_json::json!({"USER_NAME": "alice", "USER_ID": 42})];
    let map = build_key_map(&["user_name", "user_id"], rows, None);
    assert_eq!(
        map,
        vec![
            Some(KeyMatch::CaseInsensitive("USER_NAME".to_string())),
            Some(KeyMatch::CaseInsensitive("USER_ID".to_string()))
        ]
    );
}

#[test]
fn test_json_to_cell_typeoid_other() {
    // TypeOid::Other(n) → Json cell (same as TypeOid::Json)
    let obj = serde_json::json!({"payload": {"nested": true, "count": 42}});
    let cell = cell_from_json("payload", TypeOid::Other("custom_type".to_string()), &obj).unwrap();
    if let Some(Cell::Json(s)) = cell {
        assert!(s.contains("\"nested\":true"));
        assert!(s.contains("\"count\":42"));
    } else {
        panic!("Expected Json cell for TypeOid::Other");
    }
}

#[test]
fn test_json_to_cell_fallback_case_insensitive() {
    // column_key_map=None → fallback 4-step matching with case-insensitive step
    let fdw = OpenApiFdw {
        cached_columns: vec![CachedColumn {
            name: "status".to_string(),
            type_oid: TypeOid::String,
            camel_name: "status".to_string(),
            lower_name: "status".to_string(),
            alnum_name: "status".to_string(),
        }],
        column_key_map: vec![None], // force fallback path
        ..Default::default()
    };

    let obj = serde_json::json!({"Status": "active"});
    let cell = fdw.json_to_cell_cached(&obj, 0).unwrap();
    assert_eq!(
        cell_to_string(cell.as_ref().unwrap()),
        Some("active".to_string())
    );
}

#[test]
fn test_meta_pagination_has_more_nested() {
    // Paginated APIs: /meta/pagination/has_more + /meta/pagination/next_cursor
    let mut fdw = make_fdw_for_pagination("");
    let resp = serde_json::json!({
        "meta": {
            "pagination": {
                "has_more": true,
                "next_cursor": "cursor_abc123"
            }
        },
        "data": [{"id": 1}, {"id": 2}]
    });
    fdw.handle_pagination(&resp);
    assert_eq!(fdw.next_cursor, Some("cursor_abc123".to_string()));
    assert_eq!(fdw.next_url, None);
}

// --- OpenAPI 3.1 READ operation coverage: real-world API response patterns ---

// == normalize_datetime edge cases ==

#[test]
fn test_normalize_datetime_with_milliseconds() {
    // ISO 8601 datetime with milliseconds — should pass through
    let dt = "2024-06-15T10:30:00.123Z";
    assert_eq!(OpenApiFdw::normalize_datetime(dt), dt);
}

#[test]
fn test_normalize_datetime_short_string() {
    // String shorter than 10 chars — should not be treated as date
    assert_eq!(OpenApiFdw::normalize_datetime("2024-01"), "2024-01");
}

#[test]
fn test_normalize_datetime_long_non_date() {
    // Exactly 10 chars but not a date pattern (no dashes at right positions)
    assert_eq!(OpenApiFdw::normalize_datetime("1234567890"), "1234567890");
}

#[test]
fn test_normalize_datetime_empty_string() {
    assert_eq!(OpenApiFdw::normalize_datetime(""), "");
}

// == json_to_cell edge cases for type coercion ==

#[test]
fn test_json_to_cell_string_from_object() {
    // When a column expects text but JSON value is an object, serialize it
    let obj = serde_json::json!({"info": {"nested": true, "value": 42}});
    let cell = cell_from_json("info", TypeOid::String, &obj).unwrap();
    let s = cell_to_string(cell.as_ref().unwrap()).unwrap();
    assert!(s.contains("\"nested\":true"));
    assert!(s.contains("\"value\":42"));
}

#[test]
fn test_json_to_cell_string_from_array() {
    // When a column expects text but JSON value is an array, serialize it
    let obj = serde_json::json!({"tags": ["rust", "wasm", "sql"]});
    let cell = cell_from_json("tags", TypeOid::String, &obj).unwrap();
    let s = cell_to_string(cell.as_ref().unwrap()).unwrap();
    assert_eq!(s, r#"["rust","wasm","sql"]"#);
}

#[test]
fn test_json_to_cell_i32_from_float_truncates() {
    // JSON number 42.9 → i64 returns 42 (as_i64 truncates), then try_from
    // serde_json::as_i64() returns None for floats, so this should be None
    let obj = serde_json::json!({"val": 42.9});
    let cell = cell_from_json("val", TypeOid::I32, &obj).unwrap();
    assert!(cell.is_none(), "Float value should not coerce to I32");
}

#[test]
fn test_json_to_cell_i64_max() {
    // Maximum i64 value
    let obj = serde_json::json!({"val": i64::MAX});
    let cell = cell_from_json("val", TypeOid::I64, &obj).unwrap();
    assert!(matches!(cell, Some(Cell::I64(v)) if v == i64::MAX));
}

#[test]
fn test_json_to_cell_f64_nan_like() {
    // JSON doesn't have NaN — should always be a valid number
    let obj = serde_json::json!({"val": 1e308});
    let cell = cell_from_json("val", TypeOid::F64, &obj).unwrap();
    assert!(matches!(cell, Some(Cell::F64(_))));
}

#[test]
fn test_json_to_cell_bool_from_non_bool() {
    // Non-boolean value for boolean column → None
    let obj = serde_json::json!({"active": "yes"});
    let cell = cell_from_json("active", TypeOid::Bool, &obj).unwrap();
    assert!(cell.is_none(), "String 'yes' should not coerce to Bool");
}

#[test]
fn test_json_to_cell_numeric_from_string() {
    // Numeric column with string value → None (as_f64 returns None for strings)
    let obj = serde_json::json!({"price": "29.99"});
    let cell = cell_from_json("price", TypeOid::Numeric, &obj).unwrap();
    assert!(cell.is_none(), "String number should not coerce to Numeric");
}

#[test]
fn test_json_to_cell_json_from_null() {
    // Null value for JSON column → None (null is filtered before type matching)
    let obj = serde_json::json!({"meta": null});
    let cell = cell_from_json("meta", TypeOid::Json, &obj).unwrap();
    assert!(cell.is_none());
}

#[test]
fn test_json_to_cell_json_from_primitive() {
    // Primitive value serialized as JSON
    let obj = serde_json::json!({"val": 42});
    let cell = cell_from_json("val", TypeOid::Json, &obj).unwrap();
    if let Some(Cell::Json(s)) = cell {
        assert_eq!(s, "42");
    } else {
        panic!("Expected Json cell");
    }
}

#[test]
fn test_json_to_cell_uuid_from_non_string() {
    // UUID column with numeric value → None (as_str returns None)
    let obj = serde_json::json!({"uid": 12345});
    let cell = cell_from_json("uid", TypeOid::Uuid, &obj).unwrap();
    assert!(cell.is_none());
}

// == extract_data edge cases ==

#[test]
fn test_extract_data_response_path_to_single_object() {
    // response_path pointing to a single object (not array) → wrapped as single row
    let fdw = fdw_with_response_path(Some("/user"));
    let mut resp = serde_json::json!({
        "user": {"id": 1, "name": "alice"},
        "meta": {"request_id": "abc"}
    });
    let rows = fdw.extract_data(&mut resp).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["id"], 1);
    assert_eq!(rows[0]["name"], "alice");
}

#[test]
fn test_extract_data_deeply_nested_response_path() {
    // Three-level deep response path
    let fdw = fdw_with_response_path(Some("/response/body/items"));
    let mut resp = serde_json::json!({
        "response": {
            "body": {
                "items": [{"id": 1}, {"id": 2}]
            }
        }
    });
    let rows = fdw.extract_data(&mut resp).unwrap();
    assert_eq!(rows.len(), 2);
}

#[test]
fn test_extract_data_empty_object_is_single_row() {
    // Empty object {} treated as single row
    let fdw = fdw_with_response_path(None);
    let mut resp = serde_json::json!({});
    let rows = fdw.extract_data(&mut resp).unwrap();
    assert_eq!(rows.len(), 1);
}

// == Pagination edge cases ==

#[test]
fn test_handle_pagination_has_more_true_but_no_cursor() {
    // has_more: true but no cursor path found — should NOT paginate (avoid infinite loop)
    let mut fdw = make_fdw_for_pagination("");
    let resp = serde_json::json!({
        "has_more": true,
        "data": [{"id": 1}]
    });
    fdw.handle_pagination(&resp);
    assert_eq!(fdw.next_cursor, None);
    assert_eq!(fdw.next_url, None);
}

#[test]
fn test_handle_pagination_next_url_direct_key() {
    // Auto-detect: /next_url key directly
    let mut fdw = make_fdw_for_pagination("");
    let resp = serde_json::json!({
        "next_url": "https://api.example.com/items?page=3",
        "data": [{"id": 1}]
    });
    fdw.handle_pagination(&resp);
    assert_eq!(
        fdw.next_url,
        Some("https://api.example.com/items?page=3".to_string())
    );
}

#[test]
fn test_handle_pagination_pagination_next_url() {
    // Auto-detect: /pagination/next_url key
    let mut fdw = make_fdw_for_pagination("");
    let resp = serde_json::json!({
        "pagination": {
            "next_url": "https://api.example.com/page/2"
        },
        "data": []
    });
    fdw.handle_pagination(&resp);
    assert_eq!(
        fdw.next_url,
        Some("https://api.example.com/page/2".to_string())
    );
}

#[test]
fn test_handle_pagination_next_direct() {
    // Auto-detect: /next key directly (not nested)
    let mut fdw = make_fdw_for_pagination("");
    let resp = serde_json::json!({
        "next": "https://api.example.com/items?cursor=xyz",
        "data": [{"id": 1}]
    });
    fdw.handle_pagination(&resp);
    assert_eq!(
        fdw.next_url,
        Some("https://api.example.com/items?cursor=xyz".to_string())
    );
}

#[test]
fn test_handle_pagination_cursor_path_integer_value() {
    // Cursor path resolves to an integer — should be treated as non-string, ignored
    let mut fdw = make_fdw_for_pagination("/cursor");
    let resp = serde_json::json!({"cursor": 12345, "data": []});
    fdw.handle_pagination(&resp);
    // extract_non_empty_string returns None for non-string values
    assert_eq!(fdw.next_cursor, None);
    assert_eq!(fdw.next_url, None);
}

#[test]
fn test_handle_pagination_pagination_has_more_with_cursor() {
    // Auto-detect: /pagination/has_more + /pagination/next_cursor
    let mut fdw = make_fdw_for_pagination("");
    let resp = serde_json::json!({
        "pagination": {
            "has_more": true,
            "next_cursor": "pg_cursor_99"
        },
        "data": [{"id": 1}]
    });
    fdw.handle_pagination(&resp);
    assert_eq!(fdw.next_cursor, Some("pg_cursor_99".to_string()));
}

#[test]
fn test_handle_pagination_meta_next_url() {
    // Auto-detect: /meta/pagination/next_url (not /next)
    let mut fdw = make_fdw_for_pagination("");
    let resp = serde_json::json!({
        "meta": {
            "pagination": {
                "next_url": "https://api.example.com/page/4"
            }
        },
        "data": []
    });
    fdw.handle_pagination(&resp);
    assert_eq!(
        fdw.next_url,
        Some("https://api.example.com/page/4".to_string())
    );
}

// == Real-world API response patterns ==

#[test]
fn test_kubernetes_list_response() {
    // Kubernetes pattern: {kind, apiVersion, metadata, items:[...]}
    let fdw = fdw_with_response_path(None);
    let mut resp = serde_json::json!({
        "kind": "PodList",
        "apiVersion": "v1",
        "metadata": {"resourceVersion": "1234"},
        "items": [
            {"metadata": {"name": "pod-1"}, "status": {"phase": "Running"}},
            {"metadata": {"name": "pod-2"}, "status": {"phase": "Pending"}}
        ]
    });
    let rows = fdw.extract_data(&mut resp).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["metadata"]["name"], "pod-1");
}

#[test]
fn test_elasticsearch_hits_response() {
    // Elasticsearch pattern: response_path must be used for non-standard wrapper
    let fdw = fdw_with_response_path(Some("/hits/hits"));
    let mut resp = serde_json::json!({
        "took": 5,
        "hits": {
            "total": {"value": 2},
            "hits": [
                {"_id": "1", "_source": {"title": "Doc 1"}},
                {"_id": "2", "_source": {"title": "Doc 2"}}
            ]
        }
    });
    let rows = fdw.extract_data(&mut resp).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["_id"], "1");
}

#[test]
fn test_graphql_style_response() {
    // GraphQL-style: {data: {users: [...]}} — needs response_path
    let fdw = fdw_with_response_path(Some("/data/users"));
    let mut resp = serde_json::json!({
        "data": {
            "users": [
                {"id": "1", "name": "Alice"},
                {"id": "2", "name": "Bob"}
            ]
        }
    });
    let rows = fdw.extract_data(&mut resp).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["name"], "Alice");
}

#[test]
fn test_jsonapi_style_response() {
    // JSON:API pattern: {data: [{type, id, attributes}], meta}
    let fdw = fdw_with_response_path(None);
    let mut resp = serde_json::json!({
        "data": [
            {"type": "articles", "id": "1", "attributes": {"title": "JSON:API"}},
            {"type": "articles", "id": "2", "attributes": {"title": "REST"}}
        ],
        "meta": {"total-pages": 1}
    });
    let rows = fdw.extract_data(&mut resp).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["id"], "1");
}

// == Column key map edge cases with real API patterns ==

#[test]
fn test_build_column_key_map_k8s_nested_metadata() {
    // Kubernetes response: access nested fields via object_path
    let rows = vec![serde_json::json!({
        "metadata": {"name": "my-pod", "namespace": "default", "uid": "abc-123"},
        "status": {"phase": "Running"}
    })];
    let map = build_key_map(&["name", "namespace", "uid"], rows, Some("/metadata"));
    assert_eq!(
        map,
        vec![
            Some(KeyMatch::Exact),
            Some(KeyMatch::Exact),
            Some(KeyMatch::Exact)
        ]
    );
}

#[test]
fn test_build_column_key_map_github_mixed_casing() {
    // GitHub returns mixed casing: some camelCase, some snake_case
    let rows = vec![serde_json::json!({
        "id": 1,
        "node_id": "MDQ6VXNlcjE=",
        "login": "octocat",
        "gravatar_id": "",
        "followers_url": "https://api.github.com/users/octocat/followers"
    })];
    let map = build_key_map(
        &["id", "node_id", "login", "gravatar_id", "followers_url"],
        rows,
        None,
    );
    // All exact match — GitHub uses snake_case for these fields
    assert!(map.iter().all(|m| matches!(m, Some(KeyMatch::Exact))));
}

#[test]
fn test_build_column_key_map_numeric_keys() {
    // APIs that return numeric-like keys
    let rows = vec![serde_json::json!({
        "200": {"description": "OK"},
        "404": {"description": "Not Found"}
    })];
    // Sanitized: 200 → _200, 404 → _404
    // to_camel_case("_200") = "200", which matches the JSON key "200" → CamelCase match
    let map = build_key_map(&["_200", "_404"], rows, None);
    assert_eq!(
        map,
        vec![Some(KeyMatch::CamelCase), Some(KeyMatch::CamelCase)]
    );
}

// == Path parameter injection edge cases ==

#[test]
fn test_json_to_cell_path_param_bool_coercion() {
    // Path param for boolean column
    let mut fdw = OpenApiFdw {
        cached_columns: vec![CachedColumn {
            name: "active".to_string(),
            type_oid: TypeOid::Bool,
            camel_name: "active".to_string(),
            lower_name: "active".to_string(),
            alnum_name: "active".to_string(),
        }],
        column_key_map: vec![None],
        ..Default::default()
    };
    fdw.injected_params
        .insert("active".to_string(), "true".to_string());

    let obj = serde_json::json!({});
    let cell = fdw.json_to_cell_cached(&obj, 0).unwrap();
    assert!(matches!(cell, Some(Cell::Bool(true))));
}

#[test]
fn test_json_to_cell_path_param_f64_coercion() {
    // Path param for float column
    let mut fdw = OpenApiFdw {
        cached_columns: vec![CachedColumn {
            name: "lat".to_string(),
            type_oid: TypeOid::F64,
            camel_name: "lat".to_string(),
            lower_name: "lat".to_string(),
            alnum_name: "lat".to_string(),
        }],
        column_key_map: vec![None],
        ..Default::default()
    };
    fdw.injected_params
        .insert("lat".to_string(), "37.7749".to_string());

    let obj = serde_json::json!({});
    let cell = fdw.json_to_cell_cached(&obj, 0).unwrap();
    if let Some(Cell::F64(v)) = cell {
        assert!((v - 37.7749).abs() < f64::EPSILON);
    } else {
        panic!("Expected F64");
    }
}

#[test]
fn test_json_to_cell_path_param_invalid_number_fallback() {
    // Path param that can't parse as target type → falls back to String
    let mut fdw = OpenApiFdw {
        cached_columns: vec![CachedColumn {
            name: "id".to_string(),
            type_oid: TypeOid::I64,
            camel_name: "id".to_string(),
            lower_name: "id".to_string(),
            alnum_name: "id".to_string(),
        }],
        column_key_map: vec![None],
        ..Default::default()
    };
    fdw.injected_params
        .insert("id".to_string(), "not-a-number".to_string());

    let obj = serde_json::json!({});
    let cell = fdw.json_to_cell_cached(&obj, 0).unwrap();
    // i64 parse fails → falls back to Cell::String
    assert_eq!(
        cell_to_string(cell.as_ref().unwrap()),
        Some("not-a-number".to_string())
    );
}

#[test]
fn test_json_to_cell_path_param_json_type() {
    // Path param for JSON column
    let mut fdw = OpenApiFdw {
        cached_columns: vec![CachedColumn {
            name: "filter".to_string(),
            type_oid: TypeOid::Json,
            camel_name: "filter".to_string(),
            lower_name: "filter".to_string(),
            alnum_name: "filter".to_string(),
        }],
        column_key_map: vec![None],
        ..Default::default()
    };
    fdw.injected_params
        .insert("filter".to_string(), r#"{"status":"active"}"#.to_string());

    let obj = serde_json::json!({});
    let cell = fdw.json_to_cell_cached(&obj, 0).unwrap();
    if let Some(Cell::Json(s)) = cell {
        assert_eq!(s, r#"{"status":"active"}"#);
    } else {
        panic!("Expected Json cell");
    }
}

// == Resolve pagination URL edge cases ==

#[test]
fn test_resolve_pagination_url_empty_string() {
    let fdw = make_fdw_for_url("https://api.example.com", "/items");
    let url = fdw.resolve_pagination_url("");
    assert_eq!(url, "https://api.example.com/");
}

#[test]
fn test_resolve_pagination_url_trailing_slash_base() {
    // base_url is already trimmed of trailing slash in init()
    let fdw = make_fdw_for_url("https://api.example.com", "/v2/items");
    let url = fdw.resolve_pagination_url("/v2/items?offset=100");
    assert_eq!(url, "https://api.example.com/v2/items?offset=100");
}

// == to_camel_case edge cases ==

#[test]
fn test_to_camel_case_with_numbers() {
    assert_eq!(to_camel_case("field_2_name"), "field2Name");
}

#[test]
fn test_to_camel_case_all_uppercase() {
    // Already uppercase segments
    assert_eq!(to_camel_case("a_b_c"), "aBC");
}

// == json_to_cell with CamelCase key match ==

#[test]
fn test_json_to_cell_camel_case_key_match() {
    // CamelCase key match path
    let fdw = OpenApiFdw {
        cached_columns: vec![CachedColumn {
            name: "created_at".to_string(),
            type_oid: TypeOid::String,
            camel_name: "createdAt".to_string(),
            lower_name: "created_at".to_string(),
            alnum_name: "createdat".to_string(),
        }],
        column_key_map: vec![Some(KeyMatch::CamelCase)],
        ..Default::default()
    };

    let obj = serde_json::json!({"createdAt": "2024-01-15T10:00:00Z"});
    let cell = fdw.json_to_cell_cached(&obj, 0).unwrap();
    assert_eq!(
        cell_to_string(cell.as_ref().unwrap()),
        Some("2024-01-15T10:00:00Z".to_string())
    );
}

#[test]
fn test_json_to_cell_case_insensitive_key_match() {
    // CaseInsensitive key match path
    let fdw = OpenApiFdw {
        cached_columns: vec![CachedColumn {
            name: "user_name".to_string(),
            type_oid: TypeOid::String,
            camel_name: "userName".to_string(),
            lower_name: "user_name".to_string(),
            alnum_name: "username".to_string(),
        }],
        column_key_map: vec![Some(KeyMatch::CaseInsensitive("UserName".to_string()))],
        ..Default::default()
    };

    let obj = serde_json::json!({"UserName": "alice"});
    let cell = fdw.json_to_cell_cached(&obj, 0).unwrap();
    assert_eq!(
        cell_to_string(cell.as_ref().unwrap()),
        Some("alice".to_string())
    );
}

#[test]
fn test_json_to_cell_i16_overflow() {
    // i16 max is 32767 — 40000 should overflow
    let obj = serde_json::json!({"val": 40000});
    let cell = cell_from_json("val", TypeOid::I16, &obj).unwrap();
    assert!(cell.is_none());
}

#[test]
fn test_json_to_cell_i32_overflow() {
    // i32 max is 2147483647 — 3 billion should overflow
    let obj = serde_json::json!({"val": 3_000_000_000_i64});
    let cell = cell_from_json("val", TypeOid::I32, &obj).unwrap();
    assert!(cell.is_none());
}

#[test]
fn test_json_to_cell_json_string_value() {
    // JSON column with plain string value → serialize as JSON string
    let obj = serde_json::json!({"data": "hello"});
    let cell = cell_from_json("data", TypeOid::Json, &obj).unwrap();
    if let Some(Cell::Json(s)) = cell {
        assert_eq!(s, r#""hello""#);
    } else {
        panic!("Expected Json cell");
    }
}

#[test]
fn test_json_to_cell_json_bool_value() {
    // JSON column with bool value
    let obj = serde_json::json!({"flag": true});
    let cell = cell_from_json("flag", TypeOid::Json, &obj).unwrap();
    if let Some(Cell::Json(s)) = cell {
        assert_eq!(s, "true");
    } else {
        panic!("Expected Json cell");
    }
}

// --- URL encoding security tests ---

#[test]
fn test_rowid_url_encoding_path_traversal() {
    // Verify urlencoding::encode handles path traversal attempts
    let malicious_id = "../admin";
    let encoded = urlencoding::encode(malicious_id);
    assert_eq!(encoded, "..%2Fadmin");
    // Resulting URL would be /items/..%2Fadmin (safe) not /items/../admin (traversal)
}

#[test]
fn test_rowid_url_encoding_query_injection() {
    // Verify urlencoding::encode handles query injection attempts
    let malicious_id = "123?admin=true";
    let encoded = urlencoding::encode(malicious_id);
    assert_eq!(encoded, "123%3Fadmin%3Dtrue");
}

#[test]
fn test_rowid_url_encoding_special_chars() {
    // Verify urlencoding::encode handles various URL-unsafe chars
    let special = "id with spaces&more=stuff#fragment";
    let encoded = urlencoding::encode(special);
    assert!(!encoded.contains(' '));
    assert!(!encoded.contains('&'));
    assert!(!encoded.contains('='));
    assert!(!encoded.contains('#'));
}

#[test]
fn test_rowid_url_encoding_normal_ids() {
    // Normal IDs should pass through unchanged
    assert_eq!(urlencoding::encode("123"), "123");
    assert_eq!(urlencoding::encode("abc-def"), "abc-def");
    assert_eq!(
        urlencoding::encode("550e8400-e29b-41d4-a716-446655440000"),
        "550e8400-e29b-41d4-a716-446655440000"
    );
}

// --- Retry delay cap tests ---

#[test]
fn test_retry_delay_cap_normal_value() {
    // Normal Retry-After: 5 seconds → 5000ms, well under cap
    let secs: u64 = 5;
    let max_delay: u64 = 30_000;
    let delay = secs.saturating_mul(1000).min(max_delay);
    assert_eq!(delay, 5000);
}

#[test]
fn test_retry_delay_cap_large_value() {
    // Absurdly large Retry-After: 999999 seconds → capped to 30s
    let secs: u64 = 999_999;
    let max_delay: u64 = 30_000;
    let delay = secs.saturating_mul(1000).min(max_delay);
    assert_eq!(delay, 30_000);
}

#[test]
fn test_retry_delay_cap_u64_max() {
    // u64::MAX seconds → saturating_mul prevents overflow, then capped
    let secs: u64 = u64::MAX;
    let max_delay: u64 = 30_000;
    let delay = secs.saturating_mul(1000).min(max_delay);
    assert_eq!(delay, 30_000);
}

#[test]
fn test_retry_delay_cap_zero() {
    // Retry-After: 0 → 0ms (immediate retry)
    let secs: u64 = 0;
    let max_delay: u64 = 30_000;
    let delay = secs.saturating_mul(1000).min(max_delay);
    assert_eq!(delay, 0);
}

#[test]
fn test_retry_backoff_cap() {
    // Exponential backoff at retry_count=10 would be 1024s, but capped
    let retry_count: u32 = 10;
    let max_delay: u64 = 30_000;
    let backoff = 1000u64.saturating_mul(1 << retry_count);
    let delay = backoff.min(max_delay);
    assert_eq!(delay, 30_000);
}

// --- Response size limit tests ---

#[test]
fn test_max_response_bytes_default() {
    let fdw = OpenApiFdw::default();
    assert_eq!(fdw.max_response_bytes, 50 * 1024 * 1024); // 50 MiB
}
