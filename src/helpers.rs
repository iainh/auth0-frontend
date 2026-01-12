use axum::http::HeaderMap;

pub fn is_htmx_request(headers: &HeaderMap) -> bool {
    headers.get("hx-request").is_some()
}

pub fn htmx_target_is(headers: &HeaderMap, target: &str) -> bool {
    headers
        .get("hx-target")
        .and_then(|v| v.to_str().ok())
        .map(|v| v == target)
        .unwrap_or(false)
}

pub fn total_pages(total_items: usize, per_page: u32) -> u32 {
    let per_page = per_page.max(1);
    (total_items as u32).div_ceil(per_page).max(1)
}
