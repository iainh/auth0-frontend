use askama::Template;
use axum::response::{Html, IntoResponse, Response};

use crate::errors::AppResult;

pub fn render<T: Template>(template: T) -> AppResult<Response> {
    Ok(Html(template.render()?).into_response())
}

#[derive(Copy, Clone)]
pub enum ToastType {
    Success,
    Danger,
}

impl std::fmt::Display for ToastType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToastType::Success => write!(f, "success"),
            ToastType::Danger => write!(f, "danger"),
        }
    }
}

#[derive(Template)]
#[template(path = "toast.html")]
pub struct ToastTemplate {
    pub toast_type: ToastType,
    pub title: String,
    pub message: String,
}
