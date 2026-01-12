use std::sync::Arc;

use auth0_mgmt_api::ManagementClient;
use axum::{
    routing::{get, post},
    Router,
};
use tower_http::trace::TraceLayer;

use crate::routes::{applications, connections, logs, root, users};

#[derive(Clone)]
pub struct AppState {
    pub client: Arc<ManagementClient>,
}

pub fn build_app(client: ManagementClient) -> Router {
    let state = AppState {
        client: Arc::new(client),
    };

    Router::new()
        .route("/", get(root::index))
        .route("/users", get(users::list).post(users::create))
        .route(
            "/users/{id}",
            get(users::get).patch(users::update).delete(users::delete),
        )
        .route("/users/{id}/logs", get(users::get_logs))
        .route("/users/{id}/toggle-block", post(users::toggle_block))
        .route("/connections", get(connections::list))
        .route("/applications", get(applications::list))
        .route("/logs", get(logs::list))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
