use crate::common::GlobalContext;
use crate::daemon::email::{EmailSenderDaemon, Mail};
use crate::moderator::{Data, ModContext as Context, PageExtractor};
use crate::store::Store;
use axum::extract::Path;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use crate::common::AppState;

pub async fn list(
    _ctx: Context,
    store: Store,
    PageExtractor(page): PageExtractor,
) -> impl IntoResponse {
    let emails = store.list_emails(page).await.unwrap_or_default();
    Data::ok(emails).into_response()
}
pub async fn get(_ctx: Context, store: Store, Path(id): Path<i64>) -> impl IntoResponse {
    match store.get_email(id).await {
        Ok(None) => Data::fail("email not found").into_response(),
        Ok(Some(email)) => Data::ok(email).into_response(),
        Err(err)=>Data::fail(&err.to_string()).into_response(),
    }
}
#[derive(Deserialize, Debug, Clone)]
pub struct EmailForm {
    email: String,
    title: String,
    content: String,
}
#[axum::debug_handler(state = AppState)]
pub async fn send_test(
    ctx: Context,
    store: Store,
    GlobalContext(sender): GlobalContext<EmailSenderDaemon>,
    Json(form): Json<EmailForm>,
) -> impl IntoResponse {
    let uid = ctx.moderator.uid;
    let result = store.add_email_queue(uid, "", &form.email, &form.title, &form.content).await;
    match result {
        Ok(id) => {
            sender.send(Mail(id)).await;
        }
        Err(err) => {
            return Data::fail(err.to_string().as_str()).into_response();
        }
    }
    Data::done().into_response()
}
