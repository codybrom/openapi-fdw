//! Pagination state tracking and loop detection

/// Tracks pagination state across pages within a single scan.
///
/// Detects infinite loops (duplicate cursor/URL) and enforces page limits.
#[derive(Debug, Default)]
pub(crate) struct PaginationState {
    /// Cursor value for the next page (token-based pagination)
    pub(crate) next_cursor: Option<String>,
    /// Full or partial URL for the next page (link-based pagination)
    pub(crate) next_url: Option<String>,

    // Loop detection
    pub(crate) prev_cursor: Option<String>,
    pub(crate) prev_url: Option<String>,
    pub(crate) pages_fetched: usize,
}

impl PaginationState {
    /// Reset all pagination state for a new scan.
    pub(crate) fn reset(&mut self) {
        self.next_cursor = None;
        self.next_url = None;
        self.prev_cursor = None;
        self.prev_url = None;
        self.pages_fetched = 0;
    }

    /// Returns `true` when there are no more pages to fetch.
    pub(crate) fn is_exhausted(&self) -> bool {
        self.next_cursor.is_none() && self.next_url.is_none()
    }

    /// Detect a pagination loop (duplicate cursor or URL).
    ///
    /// Returns a human-readable reason if a loop is detected.
    pub(crate) fn detect_loop(&self) -> Option<&'static str> {
        if self.next_cursor.is_some() && self.next_cursor == self.prev_cursor {
            Some("duplicate cursor detected (possible infinite loop)")
        } else if self.next_url.is_some() && self.next_url == self.prev_url {
            Some("duplicate URL detected (possible infinite loop)")
        } else {
            None
        }
    }

    /// Returns `true` if the page limit has been reached.
    pub(crate) fn exceeds_limit(&self, max_pages: usize) -> bool {
        self.pages_fetched >= max_pages
    }

    /// Save current next values as previous (for loop detection) and increment page count.
    ///
    /// Call this before fetching each subsequent page.
    pub(crate) fn advance(&mut self) {
        self.prev_cursor.clone_from(&self.next_cursor);
        self.prev_url.clone_from(&self.next_url);
        self.pages_fetched += 1;
    }

    /// Record the first page after initial `make_request` in `begin_scan`.
    ///
    /// Only sets `pages_fetched = 1`. Does NOT copy `next_cursor`/`next_url`
    /// into `prev_*` — there was no cursor sent for the first page, so `prev_*`
    /// must stay `None` to avoid a false-positive loop detection.
    pub(crate) fn record_first_page(&mut self) {
        self.pages_fetched = 1;
    }

    /// Clear next-page pointers (e.g., on 404 or empty response).
    pub(crate) fn clear_next(&mut self) {
        self.next_cursor = None;
        self.next_url = None;
    }
}
