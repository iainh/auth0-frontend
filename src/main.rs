use std::sync::Arc;

use askama::Template;
use askama_axum::IntoResponse;
use auth0_mgmt_api::{
    types::{
        clients::ListClientsParams,
        connections::ListConnectionsParams,
        logs::ListLogsParams,
        users::{CreateUserRequest, ListUsersParams, UpdateUserRequest},
    },
    ManagementClient, UserId,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Redirect},
    routing::{get, post},
    Form, Router,
};
use serde::Deserialize;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

type AppState = Arc<ManagementClient>;
type Response = axum::response::Response;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let domain = std::env::var("AUTH0_DOMAIN").expect("AUTH0_DOMAIN must be set");
    let client_id = std::env::var("AUTH0_CLIENT_ID").expect("AUTH0_CLIENT_ID must be set");
    let client_secret =
        std::env::var("AUTH0_CLIENT_SECRET").expect("AUTH0_CLIENT_SECRET must be set");

    let client = ManagementClient::builder()
        .domain(&domain)
        .client_id(&client_id)
        .client_secret(&client_secret)
        .build()?;

    let state: AppState = Arc::new(client);

    let app = Router::new()
        .route("/", get(index))
        .route("/users", get(list_users).post(create_user))
        .route("/users/:id", get(get_user).patch(update_user).delete(delete_user))
        .route("/users/:id/logs", get(get_user_logs))
        .route("/users/:id/toggle-block", post(toggle_block_user))
        .route("/connections", get(list_connections))
        .route("/applications", get(list_applications))
        .route("/logs", get(list_logs))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("listening on http://localhost:3000");
    axum::serve(listener, app).await?;

    Ok(())
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate;

async fn index() -> impl IntoResponse {
    IndexTemplate
}

#[derive(Deserialize, Default)]
struct ListUsersQuery {
    page: Option<u32>,
    q: Option<String>,
    connection: Option<String>,
}

#[derive(Template)]
#[template(path = "users/list.html")]
struct UsersListTemplate {
    users: Vec<auth0_mgmt_api::types::users::User>,
    page: u32,
    total_pages: u32,
    search_query: String,
    connection: String,
    connections: Vec<String>,
}

#[derive(Template)]
#[template(path = "users/table.html")]
struct UsersTableTemplate {
    users: Vec<auth0_mgmt_api::types::users::User>,
    page: u32,
    total_pages: u32,
}

async fn list_users(
    State(client): State<AppState>,
    Query(query): Query<ListUsersQuery>,
    headers: axum::http::HeaderMap,
) -> Response {
    let page = query.page.unwrap_or(0);
    let per_page = 20;

    let params = ListUsersParams {
        page: Some(page),
        per_page: Some(per_page),
        include_totals: Some(true),
        q: query.q.clone(),
        connection: query.connection.clone(),
        search_engine: Some("v3".to_string()),
        sort: Some("created_at:-1".to_string()),
        ..Default::default()
    };

    let users = client.users().list(Some(params)).await.unwrap_or_default();
    let total_pages = (users.len() as u32 / per_page).max(1);

    let connections = get_connection_names(&client).await;

    let is_htmx = headers.get("hx-request").is_some();

    if is_htmx {
        UsersTableTemplate {
            users,
            page,
            total_pages,
        }
        .into_response()
    } else {
        UsersListTemplate {
            users,
            page,
            total_pages,
            search_query: query.q.unwrap_or_default(),
            connection: query.connection.unwrap_or_default(),
            connections,
        }
        .into_response()
    }
}

async fn get_connection_names(client: &ManagementClient) -> Vec<String> {
    client
        .connections()
        .list(None)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|c| c.name)
        .collect()
}

#[derive(Deserialize)]
struct CreateUserForm {
    email: String,
    password: String,
    connection: String,
    username: Option<String>,
    given_name: Option<String>,
    family_name: Option<String>,
    verify_email: Option<String>,
}

async fn create_user(
    State(client): State<AppState>,
    Form(form): Form<CreateUserForm>,
) -> Response {
    let name = match (&form.given_name, &form.family_name) {
        (Some(given), Some(family)) => Some(format!("{} {}", given, family)),
        (Some(given), None) => Some(given.clone()),
        (None, Some(family)) => Some(family.clone()),
        _ => None,
    };

    let request = CreateUserRequest {
        connection: form.connection,
        email: Some(form.email),
        password: Some(form.password),
        username: form.username.filter(|s| !s.is_empty()),
        given_name: form.given_name.filter(|s| !s.is_empty()),
        family_name: form.family_name.filter(|s| !s.is_empty()),
        name,
        verify_email: Some(form.verify_email.is_some()),
        ..Default::default()
    };

    match client.users().create(request).await {
        Ok(_) => {
            let users = client.users().list(None).await.unwrap_or_default();
            UsersTableTemplate {
                users,
                page: 0,
                total_pages: 1,
            }
            .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to create user: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create user").into_response()
        }
    }
}

#[derive(Template)]
#[template(path = "users/detail.html")]
struct UserDetailTemplate {
    user: auth0_mgmt_api::types::users::User,
}

async fn get_user(State(client): State<AppState>, Path(id): Path<String>) -> Response {
    match client.users().get(UserId::new(&id)).await {
        Ok(user) => UserDetailTemplate { user }.into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "User not found").into_response(),
    }
}

#[derive(Deserialize)]
struct UpdateUserForm {
    email: Option<String>,
    username: Option<String>,
    given_name: Option<String>,
    family_name: Option<String>,
    nickname: Option<String>,
    phone_number: Option<String>,
    picture: Option<String>,
    password: Option<String>,
}

#[derive(Template)]
#[template(path = "toast.html")]
struct ToastTemplate {
    toast_type: String,
    title: String,
    message: String,
}

async fn update_user(
    State(client): State<AppState>,
    Path(id): Path<String>,
    Form(form): Form<UpdateUserForm>,
) -> Response {
    let name = match (&form.given_name, &form.family_name) {
        (Some(given), Some(family)) if !given.is_empty() && !family.is_empty() => {
            Some(format!("{} {}", given, family))
        }
        (Some(given), _) if !given.is_empty() => Some(given.clone()),
        (_, Some(family)) if !family.is_empty() => Some(family.clone()),
        _ => None,
    };

    let request = UpdateUserRequest {
        email: form.email.filter(|s| !s.is_empty()),
        username: form.username.filter(|s| !s.is_empty()),
        given_name: form.given_name.filter(|s| !s.is_empty()),
        family_name: form.family_name.filter(|s| !s.is_empty()),
        nickname: form.nickname.filter(|s| !s.is_empty()),
        phone_number: form.phone_number.filter(|s| !s.is_empty()),
        picture: form.picture.filter(|s| !s.is_empty()),
        password: form.password.filter(|s| !s.is_empty()),
        name,
        ..Default::default()
    };

    match client.users().update(UserId::new(&id), request).await {
        Ok(_) => ToastTemplate {
            toast_type: "success".to_string(),
            title: "Success".to_string(),
            message: "User updated successfully".to_string(),
        }
        .into_response(),
        Err(e) => {
            tracing::error!("Failed to update user: {:?}", e);
            ToastTemplate {
                toast_type: "danger".to_string(),
                title: "Error".to_string(),
                message: "Failed to update user".to_string(),
            }
            .into_response()
        }
    }
}

async fn delete_user(State(client): State<AppState>, Path(id): Path<String>) -> Response {
    match client.users().delete(UserId::new(&id)).await {
        Ok(_) => Redirect::to("/users").into_response(),
        Err(e) => {
            tracing::error!("Failed to delete user: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to delete user").into_response()
        }
    }
}

async fn toggle_block_user(
    State(client): State<AppState>,
    Path(id): Path<String>,
    headers: axum::http::HeaderMap,
) -> Response {
    let user = match client.users().get(UserId::new(&id)).await {
        Ok(u) => u,
        Err(_) => return (StatusCode::NOT_FOUND, "User not found").into_response(),
    };

    let currently_blocked = user.blocked.unwrap_or(false);
    let request = UpdateUserRequest {
        blocked: Some(!currently_blocked),
        ..Default::default()
    };

    match client.users().update(UserId::new(&id), request).await {
        Ok(_) => {
            let is_htmx_partial = headers.get("hx-target").map(|v| v.to_str().unwrap_or("")) == Some("users-table");

            if is_htmx_partial {
                let users = client.users().list(None).await.unwrap_or_default();
                UsersTableTemplate {
                    users,
                    page: 0,
                    total_pages: 1,
                }
                .into_response()
            } else {
                Redirect::to(&format!("/users/{}", id)).into_response()
            }
        }
        Err(e) => {
            tracing::error!("Failed to toggle block status: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to update user").into_response()
        }
    }
}

#[derive(Template)]
#[template(path = "users/logs.html")]
struct UserLogsTemplate {
    logs: Vec<auth0_mgmt_api::types::logs::LogEvent>,
}

async fn get_user_logs(State(client): State<AppState>, Path(id): Path<String>) -> Response {
    let params = auth0_mgmt_api::types::users::GetUserLogsParams {
        page: Some(0),
        per_page: Some(10),
        sort: Some("date:-1".to_string()),
        include_totals: Some(false),
    };

    match client.users().get_logs(UserId::new(&id), Some(params)).await {
        Ok(logs) => UserLogsTemplate { logs }.into_response(),
        Err(_) => Html("<p class='text-muted'>Unable to load logs</p>").into_response(),
    }
}

#[derive(Template)]
#[template(path = "connections/list.html")]
struct ConnectionsListTemplate {
    connections: Vec<auth0_mgmt_api::types::connections::Connection>,
}

#[derive(Template)]
#[template(path = "connections/table.html")]
struct ConnectionsTableTemplate {
    connections: Vec<auth0_mgmt_api::types::connections::Connection>,
}

async fn list_connections(
    State(client): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Response {
    let params = ListConnectionsParams {
        page: Some(0),
        per_page: Some(100),
        include_totals: Some(false),
        ..Default::default()
    };

    let connections = client
        .connections()
        .list(Some(params))
        .await
        .unwrap_or_default();

    let is_htmx = headers.get("hx-request").is_some();

    if is_htmx {
        ConnectionsTableTemplate { connections }.into_response()
    } else {
        ConnectionsListTemplate { connections }.into_response()
    }
}

#[derive(Template)]
#[template(path = "applications/list.html")]
struct ApplicationsListTemplate {
    applications: Vec<auth0_mgmt_api::types::clients::Client>,
}

#[derive(Template)]
#[template(path = "applications/table.html")]
struct ApplicationsTableTemplate {
    applications: Vec<auth0_mgmt_api::types::clients::Client>,
}

async fn list_applications(
    State(client): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Response {
    let params = ListClientsParams {
        page: Some(0),
        per_page: Some(100),
        include_totals: Some(false),
        ..Default::default()
    };

    let applications = client.clients().list(Some(params)).await.unwrap_or_default();

    let is_htmx = headers.get("hx-request").is_some();

    if is_htmx {
        ApplicationsTableTemplate { applications }.into_response()
    } else {
        ApplicationsListTemplate { applications }.into_response()
    }
}

#[derive(Deserialize, Default)]
struct ListLogsQuery {
    page: Option<u32>,
    q: Option<String>,
}

#[derive(Template)]
#[template(path = "logs/list.html")]
struct LogsListTemplate {
    logs: Vec<auth0_mgmt_api::types::logs::LogEvent>,
    page: u32,
    total_pages: u32,
    search_query: String,
}

#[derive(Template)]
#[template(path = "logs/table.html")]
struct LogsTableTemplate {
    logs: Vec<auth0_mgmt_api::types::logs::LogEvent>,
    page: u32,
    total_pages: u32,
}

async fn list_logs(
    State(client): State<AppState>,
    Query(query): Query<ListLogsQuery>,
    headers: axum::http::HeaderMap,
) -> Response {
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

    let logs = client.logs().list(Some(params)).await.unwrap_or_default();
    let total_pages = (logs.len() as u32 / per_page).max(1);

    let is_htmx = headers.get("hx-request").is_some();

    if is_htmx {
        LogsTableTemplate {
            logs,
            page,
            total_pages,
        }
        .into_response()
    } else {
        LogsListTemplate {
            logs,
            page,
            total_pages,
            search_query: query.q.unwrap_or_default(),
        }
        .into_response()
    }
}
