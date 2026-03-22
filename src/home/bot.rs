use crate::common::AppState;
use crate::common::{GlobalContext, LocaleProvider};
use crate::daemon::notify::NotifyDaemon;
use crate::home::topic::{bake, Bake, BakeOutput};
use crate::moderator::Data;
use crate::store::topic::{CommentDisplay, CommentSearch, TopicDisplay, TopicIndex, TopicSearch};
use crate::store::{RangeQuery, Store};
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{FromRequestParts, Path, Query, State, WebSocketUpgrade};
use axum::http::request::Parts;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::StreamExt;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use tokio::select;

pub struct Context {
    store: Store,
    uid: i64,
    username: String,
}
impl LocaleProvider for Context {
    fn locale(&self) -> &str {
        "en_US"
    }
}
impl Context {
    pub fn store(&self) -> &Store {
        &self.store
    }
}
impl FromRequestParts<AppState> for Context {
    type Rejection = Json<Data<()>>;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let Some(token) = parts.headers.get("token") else {
            return Err(Data::error("missing token"));
        };
        // token 暂时使用后台管理员的，后期在区分类型
        let store = state.store();
        let Ok(Some(moderator)) = store.get_bot(token.to_str().unwrap_or_default()).await else {
            return Err(Data::error("missing token"));
        };

        Ok(Context {
            store,
            uid: moderator.uid,
            username: moderator.username,
        })
    }
}

async fn watch(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}
async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut sub = state.subscribe();
    loop {
        select! {
            ret = sub.recv() => {
                match ret {
                    Ok(msg) => {
                        let Ok(txt) =serde_json::to_string(&msg).inspect_err(|e|{
                            warn!("failed to serialize message: {}", e);
                        }) else { continue; };
                        if let Err(err) = socket.send(Message::Text(txt.into())).await {
                            warn!("websocket 已经关闭: {}", err);
                            return;
                        }
                    }
                    Err(err) => {
                        warn!("遇到我无法理解的错误: {}", err);
                        return;
                    }
                }
            },
            msg = socket.next() => {
                match msg {
                    None => {
                        return;
                    }
                    Some(msg) => {
                        info!("收到客户端指令: {:?}", msg);
                    }
                }
            }
        }
    }
}
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/watch", get(watch))
        .route("/nodes", get(list_nodes))
        .route("/nodes/{slug}", get(get_node))
        .route("/nodes/{slug}/topics", get(list_node_topics))
        .route("/topics", get(list_topics).post(create_topic))
        .route("/topics/{tid}", get(get_topic))
        .route(
            "/topics/{tid}/comments",
            get(get_topic_comments).post(create_comment),
        )
        .route("/super/move-topic", post(move_topic_handler)) // 移动帖子
        .route("/super/lock-topic", post(lock_topic_handler)) // 锁定帖子
        .route("/super/delete-topic", post(delete_topic_handler)) // 删除帖子（实际移动到回收站）
        .route("/super/delete-comment", post(delete_comment_handler)) // 删除评论（真的删除）
}

async fn list_nodes(ctx: Context) -> Json<Data<Vec<crate::store::node::Node>>> {
    match ctx.store().get_nodes().await {
        Ok(nodes) => Data::ok(nodes.into_iter().filter(|n| n.show_in_list).collect()),
        Err(err) => Data::error(&err.to_string()),
    }
}

async fn get_node(ctx: Context, Path(slug): Path<String>) -> Json<Data<crate::store::node::Node>> {
    match ctx.store().get_node(&slug).await {
        Ok(Some(node)) => Data::ok(node),
        Ok(None) => Data::error("Node not found"),
        Err(err) => Data::error(&err.to_string()),
    }
}

async fn list_node_topics(
    ctx: Context,
    Path(slug): Path<String>,
    range: RangeQuery,
) -> Json<Data<Vec<TopicIndex>>> {
    match ctx.store().get_node(&slug).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            return Data::error("Node not found");
        }
        Err(err) => return Data::error(&err.to_string()),
    };
    let search = TopicSearch {
        node_slug: Some(slug),
        recent_flag: true,
        ..Default::default()
    };
    match ctx.store().topics_before(&search, &range).await {
        Ok(topics) => Data::ok(topics),
        Err(e) => Data::error(&e.to_string()),
    }
}

async fn list_topics(ctx: Context, range: RangeQuery) -> Json<Data<Vec<TopicIndex>>> {
    let search = TopicSearch {
        recent_flag: true,
        ..Default::default()
    };
    match ctx.store().topics_before(&search, &range).await {
        Ok(topics) => Data::ok(topics),
        Err(e) => Data::error(&e.to_string()),
    }
}

async fn get_topic(ctx: Context, Path(tid): Path<i64>) -> Json<Data<TopicDisplay>> {
    match ctx.store().get_topic(tid).await {
        Ok(Some(topic)) => Data::ok(topic),
        Ok(None) => Data::error("Topic not found"),
        Err(err) => Data::error(&err.to_string()),
    }
}

// For /super/move-topic
#[derive(Deserialize)]
struct MoveTopicRequest {
    topic_id: i64,
    node_slug: String,
}
async fn move_topic_handler(ctx: Context, Json(req): Json<MoveTopicRequest>) -> Json<Data<()>> {
    let store = ctx.store();
    let Ok(Some(node)) = store.get_node(&req.node_slug).await else {
        return Data::error("Node not found");
    };
    match store.move_topic(req.topic_id, node.id).await {
        Ok(_) => Data::ok(()),
        Err(e) => Data::error(&e.to_string()),
    }
}

// For /super/lock-topic
#[derive(Deserialize)]
struct LockTopicRequest {
    topic_id: i64,
    locked: bool,
}
async fn lock_topic_handler(ctx: Context, Json(req): Json<LockTopicRequest>) -> Json<Data<()>> {
    let store = ctx.store();
    match store.lock_topic(req.topic_id, req.locked).await {
        Ok(_) => Data::ok(()),
        Err(e) => Data::error(&e.to_string()),
    }
}

// For /super/delete-topic
#[derive(Deserialize)]
struct DeleteTopicRequest {
    topic_id: i64,
}
async fn delete_topic_handler(ctx: Context, Json(req): Json<DeleteTopicRequest>) -> Json<Data<()>> {
    let store = ctx.store();
    match store.trash_topic(req.topic_id).await {
        Ok(_) => Data::ok(()),
        Err(e) => Data::error(&e.to_string()),
    }
}

// For /super/delete-comment
#[derive(Deserialize)]
struct DeleteCommentRequest {
    comment_id: i64,
}
async fn delete_comment_handler(
    ctx: Context,
    Json(req): Json<DeleteCommentRequest>,
) -> Json<Data<()>> {
    let store = ctx.store();
    match store.delete_comment(req.comment_id).await {
        Ok(_) => Data::ok(()),
        Err(e) => Data::error(&e.to_string()),
    }
}

#[derive(Deserialize)]
struct CreateTopicRequest {
    node_slug: String,
    title: String,
    content: String,
}

#[derive(Serialize)]
struct CreateTopicResponse {
    id: i64,
}

async fn create_topic(
    ctx: Context,
    GlobalContext(notify): GlobalContext<NotifyDaemon>,
    Json(req): Json<CreateTopicRequest>,
) -> Json<Data<CreateTopicResponse>> {
    let store = ctx.store();
    let Ok(Some(node)) = store.get_node(&req.node_slug).await else {
        return Data::error("Node not found");
    };
    if node.access_only {
        return Data::error("Access only");
    }
    let cfg = store.get_post_config().await;

    let BakeOutput {
        content_render,
        content_plain,
        at_users,
    } = match bake(&ctx, &cfg, Some(&req.title), &req.content) {
        Bake::Tips(msg) => {
            return Data::error(msg.as_str());
        }
        Bake::Output(data) => data,
    };
    match store
        .new_topic(
            ctx.uid,
            node.id,
            &req.title,
            &req.content,
            &content_render,
            &content_plain,
        )
        .await
    {
        Ok(id) => {
            notify.topic_at_users(id, at_users).await; // 给提及用户推送消息
            if let Err(e) = store.update_topic_bot_status(id, true).await {
                error!("Failed to update topic bot status: {}", e);
            }
            Data::ok(CreateTopicResponse { id })
        }
        Err(e) => Data::error(&e.to_string()),
    }
}

#[derive(Deserialize)]
struct CreateCommentRequest {
    content: String,
}
#[derive(Deserialize)]
struct UsernameFilter {
    username: Option<String>,
}
async fn get_topic_comments(
    ctx: Context,
    store: Store,
    Path(tid): Path<i64>,
    range: RangeQuery,
    Query(u): Query<UsernameFilter>,
) -> Json<Data<Vec<CommentDisplay>>> {
    let search = CommentSearch {
        topic: Some(tid),
        username: u.username, // 只显示某个用户的评论，用来构建 agent 上下文使用
        viewer_id: Some(ctx.uid),
    };
    match store.comments_before(&search, &range).await {
        Ok(data) => Data::ok(data),
        Err(err) => Data::error(&err.to_string()),
    }
}
#[derive(Serialize)]
struct CreateCommentResponse {
    id: i64,
}

async fn create_comment(
    ctx: Context,
    GlobalContext(notify): GlobalContext<NotifyDaemon>,
    Path(tid): Path<i64>,
    Json(req): Json<CreateCommentRequest>,
) -> Json<Data<CreateCommentResponse>> {
    let store = ctx.store();
    let cfg = store.get_post_config().await;
    let BakeOutput {
        content_render,
        content_plain,
        at_users,
    } = match bake(&ctx, &cfg, None, &req.content) {
        Bake::Tips(msg) => {
            return Data::error(msg.as_str());
        }
        Bake::Output(data) => data,
    };

    match store
        .comment(ctx.uid, tid, req.content, content_render, content_plain)
        .await
    {
        Ok(id) => {
            notify.comment_at_users(id, at_users).await; // 给提及用户推送消息
            if let Err(e) = store.update_comment_bot_status(id, true).await {
                error!("Failed to update comment bot status: {}", e);
            }
            Data::ok(CreateCommentResponse { id })
        }
        Err(e) => Data::error(&e.to_string()),
    }
}
