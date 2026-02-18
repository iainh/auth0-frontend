use askama::Template;
use auth0_mgmt_api::{
    types::users::{CreateUserRequest, ListUsersParams, UpdateUserRequest},
    UserId,
};
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use htmx_form_errors::FormErrors;
use serde::Deserialize;
use validator::Validate;

use crate::errors::{AppError, AppResult};
use crate::helpers::{htmx_target_is, is_htmx_request, total_pages};
use crate::routes::connections::get_connection_names;
use crate::state::AppState;
use crate::templates::render;

#[derive(Template)]
#[template(path = "users/list.html")]
struct ListTemplate {
    users: Vec<auth0_mgmt_api::types::users::User>,
    page: u32,
    total_pages: u32,
    search_query: String,
    connection: String,
    connections: Vec<String>,
    form: CreateForm,
    errors: FormErrors,
}

#[derive(Template)]
#[template(path = "users/table.html")]
struct TableTemplate {
    users: Vec<auth0_mgmt_api::types::users::User>,
    page: u32,
    total_pages: u32,
}

#[derive(Template)]
#[template(path = "users/detail.html")]
struct DetailTemplate {
    user: auth0_mgmt_api::types::users::User,
    errors: FormErrors,
}

#[derive(Template)]
#[template(path = "users/create_form.html")]
struct CreateFormTemplate {
    form: CreateForm,
    connections: Vec<String>,
    errors: FormErrors,
}

#[derive(Template)]
#[template(path = "users/logs.html")]
struct LogsTemplate {
    logs: Vec<auth0_mgmt_api::types::logs::LogEvent>,
}

#[derive(Deserialize, Default)]
pub struct ListQuery {
    page: Option<u32>,
    q: Option<String>,
    connection: Option<String>,
}

pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
    headers: HeaderMap,
) -> AppResult<Response> {
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

    let users = match state.client.users().list(Some(params)).await {
        Ok(users) => users,
        Err(e) => {
            tracing::error!(error = ?e, "failed to list users");
            Vec::new()
        }
    };
    let pages = total_pages(users.len(), per_page);
    let connections = get_connection_names(&state.client).await;

    if is_htmx_request(&headers) {
        render(TableTemplate {
            users,
            page,
            total_pages: pages,
        })
    } else {
        render(ListTemplate {
            users,
            page,
            total_pages: pages,
            search_query: query.q.unwrap_or_default(),
            connection: query.connection.unwrap_or_default(),
            connections,
            form: CreateForm::default(),
            errors: FormErrors::new(),
        })
    }
}

#[derive(Clone, Deserialize, Default, Validate)]
pub struct CreateForm {
    #[validate(email(message = "Must be a valid email address"))]
    email: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    password: String,
    #[validate(length(min = 1, message = "Connection is required"))]
    connection: String,
    username: Option<String>,
    given_name: Option<String>,
    family_name: Option<String>,
    verify_email: Option<String>,
}

pub async fn create(
    State(state): State<AppState>,
    Form(form): Form<CreateForm>,
) -> AppResult<Response> {
    let errors = match form.validate() {
        Ok(_) => FormErrors::new(),
        Err(e) => FormErrors::from(e),
    };

    if !errors.is_empty() {
        let connections = get_connection_names(&state.client).await;
        return render(CreateFormTemplate {
            form,
            connections,
            errors,
        });
    }

    let form_snapshot = form.clone();

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

    match state.client.users().create(request).await {
        Ok(_) => {
            let users = match state.client.users().list(None).await {
                Ok(users) => users,
                Err(e) => {
                    tracing::error!(error = ?e, "failed to list users after create");
                    Vec::new()
                }
            };

            render(TableTemplate {
                users,
                page: 0,
                total_pages: 1,
            })
        }
        Err(e) => {
            tracing::error!(error = ?e, "failed to create user");
            let mut errors = FormErrors::new();
            errors.add_base(&format!("Failed to create user: {}", e));
            let connections = get_connection_names(&state.client).await;
            render(CreateFormTemplate {
                form: form_snapshot,
                connections,
                errors,
            })
        }
    }
}

pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Response> {
    let user = state
        .client
        .users()
        .get(UserId::new(&id))
        .await
        .map_err(|e| {
            tracing::warn!(error = ?e, %id, "user not found");
            AppError::NotFound
        })?;

    render(DetailTemplate {
        user,
        errors: FormErrors::new(),
    })
}

#[derive(Deserialize, Validate)]
pub struct UpdateForm {
    #[validate(email(message = "Must be a valid email address"))]
    email: Option<String>,
    username: Option<String>,
    given_name: Option<String>,
    family_name: Option<String>,
    nickname: Option<String>,
    phone_number: Option<String>,
    #[validate(url(message = "Must be a valid URL"))]
    picture: Option<String>,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    password: Option<String>,
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Form(form): Form<UpdateForm>,
) -> AppResult<Response> {
    let errors = match form.validate() {
        Ok(_) => FormErrors::new(),
        Err(e) => FormErrors::from(e),
    };

    if !errors.is_empty() {
        let user = state
            .client
            .users()
            .get(UserId::new(&id))
            .await
            .map_err(|_| AppError::NotFound)?;
        return render(DetailTemplate { user, errors });
    }

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

    match state.client.users().update(UserId::new(&id), request).await {
        Ok(_) => {
            let user = state
                .client
                .users()
                .get(UserId::new(&id))
                .await
                .map_err(|_| AppError::NotFound)?;
            render(DetailTemplate {
                user,
                errors: FormErrors::new(),
            })
        }
        Err(e) => {
            tracing::error!(error = ?e, %id, "failed to update user");
            let mut errors = FormErrors::new();
            errors.add_base(&format!("Failed to update user: {}", e));
            let user = state
                .client
                .users()
                .get(UserId::new(&id))
                .await
                .map_err(|_| AppError::NotFound)?;
            render(DetailTemplate { user, errors })
        }
    }
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Response> {
    state
        .client
        .users()
        .delete(UserId::new(&id))
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, %id, "failed to delete user");
            AppError::Auth0(e.to_string())
        })?;

    Ok(Redirect::to("/users").into_response())
}

pub async fn toggle_block(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> AppResult<Response> {
    let user = state
        .client
        .users()
        .get(UserId::new(&id))
        .await
        .map_err(|_| AppError::NotFound)?;

    let currently_blocked = user.blocked.unwrap_or(false);
    let request = UpdateUserRequest {
        blocked: Some(!currently_blocked),
        ..Default::default()
    };

    state
        .client
        .users()
        .update(UserId::new(&id), request)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, %id, "failed to toggle block status");
            AppError::Auth0(e.to_string())
        })?;

    if htmx_target_is(&headers, "users-table") {
        let users = match state.client.users().list(None).await {
            Ok(users) => users,
            Err(e) => {
                tracing::error!(error = ?e, "failed to list users");
                Vec::new()
            }
        };
        render(TableTemplate {
            users,
            page: 0,
            total_pages: 1,
        })
    } else {
        Ok(Redirect::to(&format!("/users/{}", id)).into_response())
    }
}

pub async fn get_logs(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Response> {
    let params = auth0_mgmt_api::types::users::GetUserLogsParams {
        page: Some(0),
        per_page: Some(10),
        sort: Some("date:-1".to_string()),
        include_totals: Some(false),
    };

    match state
        .client
        .users()
        .get_logs(UserId::new(&id), Some(params))
        .await
    {
        Ok(logs) => render(LogsTemplate { logs }),
        Err(_) => Ok(Html("<p class='text-muted'>Unable to load logs</p>").into_response()),
    }
}
