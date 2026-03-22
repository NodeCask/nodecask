use crate::common::{Context, VContext, WebResponse};
use crate::common::AppState;
use crate::store::email::{EmailDetail, EmailIndex};
use crate::store::{Page, Pagination, Store};
use crate::t;
use askama::Template;
use axum::extract::Path;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use into_response_derive::TemplateResponse;

#[derive(Template, TemplateResponse)]
#[template(path = "home/debug/email-list.html")]
struct EmailListTemplate {
    ctx: VContext,
    emails: Pagination<EmailIndex>,
}

#[derive(Template, TemplateResponse)]
#[template(path = "home/debug/email-view.html")]
struct EmailViewTemplate {
    ctx: VContext,
    email: EmailDetail,
}

#[axum::debug_handler(state = AppState)]
async fn get_email_list(
    ctx: Context,
    store: Store,
    page: Page,
) -> WebResponse {
    let emails = store.list_emails(page.list()).await?;
    Ok(EmailListTemplate {
        ctx: ctx.context("Email List".to_string()),
        emails,
    }
    .into_response())
}

#[axum::debug_handler(state = AppState)]
async fn get_email_detail(ctx: Context, store: Store, Path(id): Path<i64>) -> WebResponse {
    let email = store.get_email(id).await?;
    match email {
        Some(email) => Ok(EmailViewTemplate {
            ctx: ctx.context(format!("Email #{}", id)),
            email,
        }
        .into_response()),
        None => Ok(ctx.not_found().into_response()),
    }
}

pub fn router() -> Router<AppState> {
    if !std::env::var("DEVELOPER_MODE")
        .ok()
        .and_then(|s| parse_loose_bool(s.as_str()))
        .unwrap_or_default()
    {
        return Router::new();
    }
    Router::new()
        .route("/emails", get(get_email_list))
        .route("/emails/{id}", get(get_email_detail))
}

fn parse_loose_bool(s: &str) -> Option<bool> {
    match s.trim().to_lowercase().as_str() {
        "1" | "y" | "yes" | "on" | "enable" | "true" => Some(true),
        "0" | "n" | "no" | "off" | "disable" | "false" => Some(false),
        _ => None,
    }
}
