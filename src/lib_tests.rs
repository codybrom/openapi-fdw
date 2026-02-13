use super::*;

// --- Cross-cutting default tests ---

#[test]
fn test_max_response_bytes_default() {
    let fdw = OpenApiFdw::default();
    assert_eq!(fdw.config.max_response_bytes, 50 * 1024 * 1024); // 50 MiB
}

#[test]
fn test_pagination_safety_defaults() {
    let fdw = OpenApiFdw::default();
    assert_eq!(fdw.config.max_pages, 1000);
    assert_eq!(fdw.pagination.pages_fetched, 0);
    assert_eq!(fdw.pagination.prev_cursor, None);
    assert_eq!(fdw.pagination.prev_url, None);
}
