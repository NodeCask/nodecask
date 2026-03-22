use crate::moderator::{Data, ModContext};
use axum::extract::Path;
use axum::Json;
use serde::Deserialize;

pub async fn list(ctx: ModContext) -> Json<Data<Vec<crate::store::access_token::AccessToken>>> {
    match ctx.store.list_access_tokens("bot").await {
        Ok(tokens) => Data::ok(tokens),
        Err(e) => Data::error(&e.to_string()),
    }
}

#[derive(Deserialize)]
pub struct CreateTokenRequest {
    description: String,
    user_id: Option<i64>,
}

pub async fn create(ctx: ModContext, Json(req): Json<CreateTokenRequest>) -> Json<Data<String>> {
    let user_id = if let Some(uid) = req.user_id {
        // Verify user exists
        if let Err(_) = ctx.store.get_user_detail(uid).await {
             return Data::error("User not found");
        }
        uid
    } else {
        ctx.moderator.uid
    };

    match ctx
        .store
        .create_access_token(user_id, &req.description, "bot")
        .await
    {
        Ok(token) => Data::ok(token),
        Err(e) => Data::error(&e.to_string()),
    }
}

pub async fn delete(ctx: ModContext, Path(token): Path<String>) -> Json<Data<()>> {
    match ctx.store.delete_access_token(&token).await {
        Ok(_) => Data::done(),
        Err(e) => Data::error(&e.to_string()),
    }
}
