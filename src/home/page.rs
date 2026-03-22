use crate::common::Context;
use crate::common::AppState;
use crate::store::Store;
use crate::t;
use askama::Template;
use axum::body::Body;
use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode, Uri};
use axum::response::IntoResponse;
use axum::Json;
use into_response_derive::TemplateResponse;
use object_store::path::Path;
use object_store::ObjectStoreExt;
use percent_encoding::percent_decode_str;
use serde_json::json;

#[derive(Template, TemplateResponse)]
#[template(path = "home/page.html")]
struct CustomPageTemplate {
    ctx: crate::common::VContext,
    content: String,
}

#[derive(Template, TemplateResponse)]
#[template(path = "home/error/404.html")]
pub struct NotFoundTemplate {
    pub ctx: crate::common::VContext,
}

#[derive(Template, TemplateResponse)]
#[template(path = "home/error/500.html")]
pub struct InternalErrorTemplate {
    pub ctx: crate::common::VContext,
}

#[axum::debug_handler(state = AppState)]
pub async fn page(State(state):State<AppState>,ctx: Context, store: Store, headers: HeaderMap, uri: Uri) -> impl IntoResponse {
    let accept = headers
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    let decoded_path = percent_decode_str(uri.path()).decode_utf8_lossy();

    // 优先从 page 表格查询是否存在自定义页面
    if accept.is_empty()
        || accept.contains("text/html")
        || accept.contains("application/xhtml+xml")
        || accept.contains("application/xml")
    {
        // Try to find a custom page for this path (slug)
        // Extract slug from path: /page-slug -> page-slug
        let path = decoded_path.trim_start_matches("/");

        if let Some(page) = store.get_page(path).await {
            return CustomPageTemplate {
                ctx: ctx.context(page.title.clone()),
                content: page.get_render_content(),
            }
            .into_response();
        }
    }

    // 从存储桶里面查询是否存在对应文件
    if let Ok(location) = Path::parse(&*decoded_path){
        match state.fs.get(&location).await {
            Ok(ret) => {
                let size = ret.meta.size;
                let stream = ret.into_stream();
                let mime = mime_guess::from_path(&*decoded_path).first_or_octet_stream();
                let body = Body::from_stream(stream);

                let mut headers = HeaderMap::new();
                if let Ok(val) = axum::http::HeaderValue::from_str(mime.as_ref()) {
                    headers.insert(header::CONTENT_TYPE, val);
                }
                if let Ok(val) = axum::http::HeaderValue::from_str(&size.to_string()) {
                    headers.insert(header::CONTENT_LENGTH, val);
                }

                return (headers, body).into_response();
            }
            Err(_err) => {
                // 一般来说这里不需要处理错误
            }
        }
    }
    // 根据请求类型，返回对应 404 错误
    if accept.is_empty()
        || accept.contains("text/html")
        || accept.contains("application/xhtml+xml")
        || accept.contains("application/xml")
    {
        return (StatusCode::NOT_FOUND, ctx.not_found()).into_response();
    }
    if accept.contains("application/json") {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "code": 404,
                "error": "Not Found",
                "message": format!("The path '{}' does not exist", decoded_path)
            })),
        )
            .into_response();
    }
    StatusCode::NOT_FOUND.into_response()
}
