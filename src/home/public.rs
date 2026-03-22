use crate::common::{AppState, Context, OptionalCurrentUser, VContext};
use crate::store::topic::TopicIndex;
use crate::store::Store;
use crate::t;
use crate::t_owned;
use askama::Template;
use axum::body::Body;
use axum::extract::Query;
use axum::http::header::CONTENT_TYPE;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect};
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use into_response_derive::TemplateResponse;
use log::warn;
use serde::Deserialize;
use sqlx::types::time::OffsetDateTime;
use std::io::{Cursor, Read};
use tantivy::time::Duration;

#[derive(Template, TemplateResponse)]
#[template(path = "home/index.html")]
pub struct IndexTemplate {
    ctx: VContext,
    topics: Vec<TopicIndex>,
}

#[axum::debug_handler(state = AppState)]
pub async fn robots(store: Store) -> impl IntoResponse {
    #[derive(Deserialize)]
    struct Content {
        content: String,
    }
    let content = store.get_cfg::<Content>("robots.txt").await;
    match content {
        None => StatusCode::NOT_FOUND.into_response(),
        Some(content) => ([(CONTENT_TYPE, "text/plain")], content.content).into_response(),
    }
}
#[axum::debug_handler(state = AppState)]
pub async fn index(ctx: Context, store: Store, user: OptionalCurrentUser) -> impl IntoResponse {
    let topics = match store.discover(user.as_ref().map(|u| u.id)).await {
        Ok(data) => data,
        Err(err) => {
            return ctx.error(&err.to_string()).into_response();
        }
    };
    let t = IndexTemplate {
        ctx: ctx.context(t_owned!(ctx, "home.home")),
        topics,
    };
    t.into_response()
}
// preferred-theme

#[derive(Debug, Deserialize)]
pub struct PreferredTheme {
    theme: Option<String>,
    back: Option<String>,
}
#[axum::debug_handler(state = AppState)]
pub async fn select_theme(
    store: Store,
    user: OptionalCurrentUser,
    headers: HeaderMap,
    Query(pref): Query<PreferredTheme>,
    jar: CookieJar,
) -> impl IntoResponse {
    let Some(theme) = pref.theme else {
        return Redirect::to("/").into_response();
    };

    let e: OffsetDateTime = OffsetDateTime::now_utc() + Duration::days(30);
    let jar = jar.add(Cookie::build(("theme", theme.clone())).expires(e));
    if let Some(user) = user {
        let value = serde_json::json!({
            "theme": theme.clone()
        });
        if let Err(err) = store
            .set_user_attr(user.id, "preferred-theme", Some(&value))
            .await
        {
            warn!("Failed to update user preferred theme: {}", err)
        }
    }
    if let Some(url) = &pref.back {
        (jar, Redirect::to(url.as_str())).into_response()
    } else {
        let referer = headers.get("referer").and_then(|h| h.to_str().ok()).unwrap_or("/");
        (jar, Redirect::to(referer)).into_response()
    }
}
const PUBLIC_ZIP: &[u8] = include_bytes!("../../public.zip");
pub async fn public_handler(
    axum::extract::path::Path(path): axum::extract::path::Path<String>,
) -> impl IntoResponse {
    let mut archive = zip::ZipArchive::new(Cursor::new(PUBLIC_ZIP)).unwrap();
    let path = path.trim_start_matches('/');

    match archive.by_name(path) {
        Ok(mut file) => {
            let mut content = Vec::new();
            if file.read_to_end(&mut content).is_err() {
                return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read file").into_response();
            }

            let mime = mime_guess::from_path(path).first_or_octet_stream();

            (
                [
                    (header::CONTENT_TYPE, mime.as_ref()),
                    (header::CACHE_CONTROL, "public, max-age=3600"),
                ],
                Body::from(content),
            )
                .into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, "File not found").into_response(),
    }
}
