use askama::Template;
use auth0_mgmt_api::types::clients::ListClientsParams;
use axum::{extract::State, http::HeaderMap, response::Response};

use crate::errors::AppResult;
use crate::helpers::is_htmx_request;
use crate::state::AppState;
use crate::templates::render;

#[derive(Template)]
#[template(path = "applications/list.html")]
struct ListTemplate {
    applications: Vec<auth0_mgmt_api::types::clients::Client>,
}

#[derive(Template)]
#[template(path = "applications/table.html")]
struct TableTemplate {
    applications: Vec<auth0_mgmt_api::types::clients::Client>,
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Response> {
    let params = ListClientsParams {
        page: Some(0),
        per_page: Some(100),
        include_totals: Some(false),
        ..Default::default()
    };

    let applications = match state.client.clients().list(Some(params)).await {
        Ok(applications) => applications,
        Err(e) => {
            tracing::error!(error = ?e, "failed to list applications");
            Vec::new()
        }
    };

    if is_htmx_request(&headers) {
        render(TableTemplate { applications })
    } else {
        render(ListTemplate { applications })
    }
}
