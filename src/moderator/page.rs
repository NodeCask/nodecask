use crate::moderator::{Data, ModContext as Context};
use axum::extract::Path;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

pub async fn list(Context { store, .. }: Context) -> impl IntoResponse {
    let pages = store.get_pages().await;
    Data::ok(pages).into_response()
}

#[derive(Deserialize)]
pub struct CreateForm {
    pub path: String,
    pub title: String,
    pub description: String,
    pub content_type: String,
    pub content: String,
}

pub async fn create(
    Context { store, .. }: Context,
    Json(form): Json<CreateForm>,
) -> impl IntoResponse {
    match store
        .create_page(
            &form.path,
            &form.title,
            &form.description,
            &form.content_type,
            &form.content,
        )
        .await
    {
        Ok(_) => Data::done().into_response(),
        Err(e) => Data::fail(&e.to_string()).into_response(),
    }
}

pub async fn update(
    Context { store, .. }: Context,
    Path(id): Path<i64>,
    Json(form): Json<CreateForm>,
) -> impl IntoResponse {
    let path = form.path.trim().trim_start_matches("/");
    match store
        .update_page(
            id,
            path,
            &form.title,
            &form.description,
            &form.content_type,
            &form.content,
        )
        .await
    {
        Ok(_) => Data::done().into_response(),
        Err(e) => Data::fail(&e.to_string()).into_response(),
    }
}

pub async fn delete(Context { store, .. }: Context, Path(id): Path<i64>) -> impl IntoResponse {
    match store.delete_page(id).await {
        Ok(_) => Data::done().into_response(),
        Err(e) => Data::fail(&e.to_string()).into_response(),
    }
}