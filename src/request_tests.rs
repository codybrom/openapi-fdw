use super::*;
use crate::config::ServerConfig;

// --- resolve_pagination_url tests ---

fn make_fdw_for_url(base_url: &str, endpoint: &str) -> OpenApiFdw {
    OpenApiFdw {
        config: ServerConfig {
            base_url: base_url.to_string(),
            ..Default::default()
        },
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
