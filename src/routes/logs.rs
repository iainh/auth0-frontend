use askama::Template;
use auth0_mgmt_api::types::logs::ListLogsParams;
use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::Response,
};
use serde::Deserialize;

use crate::errors::AppResult;
use crate::helpers::{is_htmx_request, total_pages};
use crate::state::AppState;
use crate::templates::render;

#[derive(Template)]
#[template(path = "logs/list.html")]
struct ListTemplate {
    logs: Vec<auth0_mgmt_api::types::logs::LogEvent>,
    page: u32,
    total_pages: u32,
    search_query: String,
}

#[derive(Template)]
#[template(path = "logs/table.html")]
struct TableTemplate {
    logs: Vec<auth0_mgmt_api::types::logs::LogEvent>,
    page: u32,
    total_pages: u32,
}

#[derive(Deserialize, Default)]
pub struct ListQuery {
    page: Option<u32>,
    q: Option<String>,
}

pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
    headers: HeaderMap,
) -> AppResult<Response> {
    let page = query.page.unwrap_or(0);
    let per_page = 50;

    let params = ListLogsParams {
        page: Some(page),
        per_page: Some(per_page),
        include_totals: Some(true),
        q: query.q.clone(),
        sort: Some("date:-1".to_string()),
        ..Default::default()
    };

    let logs = match state.client.logs().list(Some(params)).await {
        Ok(logs) => logs,
        Err(e) => {
            tracing::error!(error = ?e, "failed to list logs");
            Vec::new()
        }
    };
    let pages = total_pages(logs.len(), per_page);

    if is_htmx_request(&headers) {
        render(TableTemplate {
            logs,
            page,
            total_pages: pages,
        })
    } else {
        render(ListTemplate {
            logs,
            page,
            total_pages: pages,
            search_query: query.q.unwrap_or_default(),
        })
    }
}
