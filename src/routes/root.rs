use askama::Template;
use axum::response::Response;

use crate::errors::AppResult;
use crate::templates::render;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate;

pub async fn index() -> AppResult<Response> {
    render(IndexTemplate)
}
