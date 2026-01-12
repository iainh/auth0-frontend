use askama::Template;
use auth0_mgmt_api::{types::connections::ListConnectionsParams, ManagementClient};
use axum::{extract::State, http::HeaderMap, response::Response};

use crate::errors::AppResult;
use crate::helpers::is_htmx_request;
use crate::state::AppState;
use crate::templates::render;

#[derive(Template)]
#[template(path = "connections/list.html")]
struct ListTemplate {
    connections: Vec<auth0_mgmt_api::types::connections::Connection>,
}

#[derive(Template)]
#[template(path = "connections/table.html")]
struct TableTemplate {
    connections: Vec<auth0_mgmt_api::types::connections::Connection>,
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Response> {
    let params = ListConnectionsParams {
        page: Some(0),
        per_page: Some(100),
        include_totals: Some(false),
        ..Default::default()
    };

    let connections = match state.client.connections().list(Some(params)).await {
        Ok(connections) => connections,
        Err(e) => {
            tracing::error!(error = ?e, "failed to list connections");
            Vec::new()
        }
    };

    if is_htmx_request(&headers) {
        render(TableTemplate { connections })
    } else {
        render(ListTemplate { connections })
    }
}

pub async fn get_connection_names(client: &ManagementClient) -> Vec<String> {
    match client.connections().list(None).await {
        Ok(connections) => connections.into_iter().map(|c| c.name).collect(),
        Err(e) => {
            tracing::error!(error = ?e, "failed to get connection names");
            Vec::new()
        }
    }
}
