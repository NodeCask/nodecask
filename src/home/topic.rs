use crate::common::{
    Broadcast, Context, CurrentUser, GlobalContext, LocaleProvider, Message, OptionalCurrentUser, WebResponse,
};
use crate::daemon::link_filter::LinkFilter;
use crate::daemon::moderate::{ModerateDaemon, Task};
use crate::daemon::notify::{Notify, NotifyDaemon};
use crate::daemon::tantivy::{SearchDaemon, TopicItem};
use crate::home::{AppState, RedirectTemplate};
use crate::markdown::render_markdown_with_mentions;
use crate::store::node::Node;
use crate::store::system::PostConfig;
use crate::store::topic::{CommentDisplay, CommentSearch, TopicDisplay, TopicIndex, TopicSearch};
use crate::store::{Page, Pagination, Store};
use crate::{t, t_owned};
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect};
use axum::Form;
use chrono::Utc;
use html2text::render::TrivialDecorator;
use into_response_derive::TemplateResponse;
use log::warn;
use rss::{ChannelBuilder, Guid, ItemBuilder};
use serde::Deserialize;

#[derive(Template, TemplateResponse)]
#[template(path = "common/login_tips.html")]
struct LoginTipsTemplate {
    ctx: crate::common::VContext,
}

#[derive(Template, TemplateResponse)]
#[template(path = "home/topic/detail.html")]
struct TopicDetailTemplate {
    topic: TopicDisplay,
    node: Node,
    comments: Pagination<CommentDisplay>,
    current_user: OptionalCurrentUser,
    turnstile: Option<(String, String)>,
    token: String, //comment token
    ctx: crate::common::VContext,
}
#[derive(Deserialize)]
pub struct SearchParams {
    q: Option<String>,
    username: Option<String>,
    node: Option<String>,
    p: Option<u32>,
}
#[derive(Template, TemplateResponse)]
#[template(path = "home/search.html")]
struct SearchTemplate {
    topics: Vec<TopicItem>,
    q: String,
    ctx: crate::common::VContext,
}
#[derive(Template, TemplateResponse)]
#[template(path = "home/topic/create.html")]
pub struct CreatePostTemplate {
    pub tips: Vec<String>,
    pub node: Node,
    pub token: String,
    pub title: String,
    pub content: String,
    pub turnstile: Option<(String, String)>,
    pub ctx: crate::common::VContext,
}

#[derive(Template, TemplateResponse)]
#[template(path = "home/topic/edit.html")]
pub struct EditPostTemplate {
    tips: Vec<String>,
    node: Node,
    topic: TopicDisplay,
    token: String,
    ctx: crate::common::VContext,
}

#[derive(Template, TemplateResponse)]
#[template(path = "home/topic/delete.html")]
pub struct DeleteConfirmTemplate {
    topic: TopicDisplay,
    token: String,
    ctx: crate::common::VContext,
}

#[derive(Template, TemplateResponse)]
#[template(path = "home/topic/lock.html")]
pub struct LockConfirmTemplate {
    topic: TopicDisplay,
    token: String,
    ctx: crate::common::VContext,
}

#[axum::debug_handler(state = AppState)]
pub async fn search(
    ctx: Context,
    _store: Store,
    State(_state): State<AppState>,
    GlobalContext(search): GlobalContext<SearchDaemon>,
    _user: OptionalCurrentUser,
    Query(params): Query<SearchParams>,
) -> WebResponse {
    let q = params.q.clone();
    let topics: Vec<TopicItem> = match params.q {
        None => vec![],
        Some(txt) => match search.search(txt).await {
            Ok(data) => data,
            Err(err) => {
                return ctx
                    .error(&format!("{}: {}", t!(ctx, "topic.system_error"), err))
                    .into();
            }
        },
    };

    let t = SearchTemplate {
        topics,
        q: q.clone().unwrap_or_default(),
        ctx: ctx.context(t_owned!(ctx, "topic.search")),
    };
    t.into()
}

#[derive(Deserialize)]
pub struct CreatePostForm {
    title: String,
    content: String,
    token: String,
    #[serde(alias = "cf-turnstile-response")]
    pub turnstile_response: Option<String>,
}
pub async fn create_form(ctx: Context, store: Store, Path(slug): Path<String>) -> WebResponse {
    if !ctx.is_login() {
        return LoginTipsTemplate {
            ctx: ctx.context(t_owned!(ctx, "topic.login")),
        }
        .into();
    }
    let Some(node) = store.get_node(&slug).await.map_err(ctx.err())? else {
        return ctx.error(t!(ctx, "topic.node_not_found")).into();
    };

    if node.moderator_access_required {
        let is_mod = ctx
            .current_user
            .as_ref()
            .map(|u| u.is_moderator)
            .unwrap_or(false);
        if !is_mod {
            return ctx.error(t!(ctx, "topic.mod_only_post")).into();
        }
    }

    if node.access_only {
        return ctx.error(t!(ctx, "topic.node_readonly")).into();
    }

    let Some(uid) = ctx.get_uid() else {
        return ctx.error(t!(ctx, "topic.system_error")).into();
    };

    let turnstile_cfg = store.get_turnstile_config().await;
    let turnstile = turnstile_cfg.get_site_key("topic");

    let t = CreatePostTemplate {
        tips: vec![],
        token: store.get_token(uid, "new-topic").await.unwrap_or_default(),
        node,
        title: "".to_string(),
        content: "".to_string(),
        turnstile,
        ctx: ctx.context(t_owned!(ctx, "topic.new_topic")),
    };
    t.into()
}

pub(crate) enum Bake {
    Tips(String),
    Output(BakeOutput),
}
pub(crate) struct BakeOutput {
    pub content_render: String,
    pub content_plain: String,
    pub at_users: Vec<String>,
}
pub(crate) fn bake(
    ctx: impl LocaleProvider,
    cfg: &PostConfig,
    title: Option<&str>,
    content: &str,
) -> Bake {
    fn tips(msg: String) -> Bake {
        Bake::Tips(msg)
    }
    if let Some(title) = title {
        // 发帖编辑帖子
        if title.chars().count() < cfg.min_title_length as usize {
            return tips(t_owned!(
                ctx,
                "topic.title_min_length",
                min = cfg.min_title_length
            ));
        }
        if title.chars().count() > cfg.max_title_length as usize {
            return tips(t_owned!(
                ctx,
                "topic.title_max_length",
                max = cfg.max_title_length
            ));
        }
        if content.chars().count() < cfg.min_content_length as usize {
            return tips(t_owned!(
                ctx,
                "topic.content_min_length",
                min = cfg.min_content_length
            ));
        }
        if content.chars().count() > cfg.max_content_length as usize {
            return tips(t_owned!(
                ctx,
                "topic.content_max_length",
                max = cfg.max_content_length
            ));
        }
    } else {
        // 回复帖子
        if content.chars().count() < cfg.min_reply_length as usize {
            return tips(t_owned!(
                ctx,
                "topic.reply_min_length",
                min = cfg.min_reply_length
            ));
        }
        if content.chars().count() > cfg.max_reply_length as usize {
            return tips(t_owned!(
                ctx,
                "topic.reply_max_length",
                max = cfg.max_reply_length
            ));
        }
    }
    for word in &cfg.sensitive_words {
        if title.map(|title| title.contains(word)).unwrap_or_default() || content.contains(word) {
            return tips(t_owned!(ctx, "topic.sensitive_word", word = word));
        }
    }
    let (content_render, mut at_users) = render_markdown_with_mentions(content);
    let content_plain = plain(content_render.as_bytes()).unwrap_or(content.to_string());

    // 同样渲染后的文本再检测一次敏感词
    if let Some(word) = cfg
        .sensitive_words
        .iter()
        .filter(|word| content_plain.contains(word.as_str()))
        .next()
    {
        return tips(t_owned!(ctx, "topic.contains_sensitive_word", word = word));
    }
    at_users.dedup();
    Bake::Output(BakeOutput {
        content_render,
        content_plain,
        at_users,
    })
}
#[axum::debug_handler(state = AppState)]
pub async fn create_post(
    ctx: Context,
    store: Store,
    broadcast: Broadcast,
    GlobalContext(moderator): GlobalContext<ModerateDaemon>,
    GlobalContext(notify): GlobalContext<NotifyDaemon>,
    GlobalContext(search): GlobalContext<SearchDaemon>,
    current_user: CurrentUser,
    Path(slug): Path<String>,
    Form(form): Form<CreatePostForm>,
) -> WebResponse {
    if !ctx.is_login() {
        return LoginTipsTemplate {
            ctx: ctx.context(t_owned!(ctx, "topic.not_logged_in")),
        }
        .into();
    }
    let Some(node) = store.get_node(&slug).await.map_err(ctx.err())? else {
        return ctx.error(t!(ctx, "topic.node_not_found")).into();
    };

    if node.moderator_access_required && !ctx.is_moderator() {
        return ctx.error(t!(ctx, "topic.mod_only_post")).into();
    }

    if node.access_only {
        return ctx.error(t!(ctx, "topic.node_readonly")).into();
    }

    let Some(uid) = ctx.get_uid() else {
        return ctx.error("system error").into();
    };

    let turnstile_cfg = store.get_turnstile_config().await;
    let turnstile = turnstile_cfg.get_site_key("topic");

    let cfg = store.get_post_config().await;
    let placeholder = CreatePostTemplate {
        tips: vec![],
        token: store.get_token(uid, "new-topic").await.unwrap_or_default(),
        node: node.clone(),
        title: form.title.clone(),
        content: form.content.clone(),
        ctx: ctx.context(t_owned!(ctx, "topic.new_topic")),
        turnstile: turnstile.clone(),
    };

    match store
        .verify_token(uid, "new-topic", form.token.as_str())
        .await
        .map_err(ctx.err())?
    {
        true => {}
        false => {
            return CreatePostTemplate {
                tips: vec![t_owned!(ctx, "topic.form_expired")],
                ..placeholder
            }
            .into();
        }
    }
    if turnstile.is_some() {
        let Some(turnstile_response) = form.turnstile_response.clone().filter(|k| !k.is_empty())
        else {
            return CreatePostTemplate {
                tips: vec![t_owned!(ctx, "auth.captcha_required")],
                ..placeholder
            }
            .into();
        };

        match turnstile_cfg.validate("topic", &turnstile_response).await {
            Ok(true) => {}
            Ok(false) => {
                return CreatePostTemplate {
                    tips: vec![t_owned!(ctx, "auth.captcha_failed")],
                    ..placeholder
                }
                .into();
            }
            Err(err) => {
                return CreatePostTemplate {
                    tips: vec![t_owned!(ctx, "auth.captcha_error", err = err)],
                    ..placeholder
                }
                .into();
            }
        }
    }

    let BakeOutput {
        content_render,
        content_plain,
        at_users,
    } = match bake(&ctx, &cfg, Some(&form.title), &form.content) {
        Bake::Tips(msg) => {
            return CreatePostTemplate {
                tips: vec![msg],
                ..placeholder
            }
            .into();
        }
        Bake::Output(data) => data,
    };

    if cfg.min_reg_age_secs > 0 {
        if let Some(user) = store
            .get_user(&ctx.current_user.as_ref().unwrap().username)
            .await
            .map_err(ctx.err())?
        {
            let created_at = user.created_at;
            let now = Utc::now();
            let age_secs = now.signed_duration_since(created_at).num_seconds();
            if age_secs < cfg.min_reg_age_secs {
                return CreatePostTemplate {
                    tips: vec![t_owned!(
                        ctx,
                        "topic.reg_age_limit",
                        secs = cfg.min_reg_age_secs
                    )],
                    ..placeholder
                }
                .into();
            }
        }
    }

    let user_detail = store
        .get_user_detail(current_user.id)
        .await
        .map_err(ctx.err())?;
    if user_detail.credit_score <= 0 {
        return CreatePostTemplate {
            tips: vec![t_owned!(ctx, "topic.account_abnormal")],
            ..placeholder
        }
        .into();
    }

    if node.topic_reward < 0 && user_detail.coins + node.topic_reward < 0 {
        return CreatePostTemplate {
            tips: vec![t_owned!(ctx, "topic.coins_insufficient")],
            ..placeholder
        }
        .into();
    }

    let id = store
        .new_topic(
            uid,
            node.id,
            &form.title,
            &form.content,
            &content_render,
            &content_plain,
        )
        .await
        .map_err(ctx.err())?;
    notify.topic_at_users(id, at_users.clone()).await; // 给提及用户推送消息

    // 更新金币
    if node.topic_reward != 0 {
        if let Err(e) = store.update_coins(uid, node.topic_reward).await {
            log::error!("Failed to update coins for user {}: {}", uid, e);
        }
    }

    moderator.push(Task::NewTopic(id)).await;
    search.update_topic_idx(id).await; // 构建全文索引
    // 广播发帖事件
    broadcast.send_silent(Message::NewTopic {
        id,
        title: form.title.clone(),
        username: current_user.username.clone(),
        node: node.slug.clone(),
    });
    // 广播 At 用户事件
    for x in at_users {
        broadcast.send_silent(Message::TopicAt {
            id,
            username: x,
            content: content_plain.clone(),
        });
    }

    RedirectTemplate {
        tips: &t!(ctx, "topic.post_success"),
        url: &format!("/t/{}", id),
        ctx: ctx.context(t_owned!(ctx, "topic.post_success")),
    }
    .into()
}

#[axum::debug_handler(state = AppState)]
pub async fn edit_form(
    ctx: Context,
    store: Store,
    current_user: CurrentUser,
    Path(id): Path<i64>,
) -> WebResponse {
    let uid = current_user.id;
    let Some(topic) = store.get_topic(id).await.map_err(ctx.err())? else {
        return ctx.error(t!(ctx, "topic.topic_not_found")).into();
    };
    let Some(node) = store.get_node(&topic.node_slug).await.map_err(ctx.err())? else {
        return ctx.error(t!(ctx, "topic.node_removed")).into();
    };

    if !can_edit(&topic, uid, &store).await {
        return ctx.error(t!(ctx, "topic.no_edit_permission")).into();
    }

    let t = EditPostTemplate {
        tips: vec![],
        token: store.get_token(uid, "edit-topic").await.unwrap_or_default(),
        topic,
        node,
        ctx: ctx.context(t_owned!(ctx, "topic.edit_topic")),
    };
    t.into()
}

#[derive(Deserialize)]
pub struct EditPostForm {
    title: String,
    content: String,
    token: String,
}

#[axum::debug_handler(state = AppState)]
pub async fn edit_post(
    ctx: Context,
    store: Store,
    GlobalContext(moderator): GlobalContext<ModerateDaemon>,
    GlobalContext(notify): GlobalContext<NotifyDaemon>,
    GlobalContext(search): GlobalContext<SearchDaemon>,
    current_user: CurrentUser,
    Path(id): Path<i64>,
    Form(form): Form<EditPostForm>,
) -> WebResponse {
    let uid = current_user.id;
    let Some(topic) = store.get_topic(id).await.map_err(ctx.err())? else {
        return ctx.error(t!(ctx, "topic.topic_not_found")).into();
    };
    let Some(node) = store.get_node(&topic.node_slug).await.map_err(ctx.err())? else {
        return ctx.error(t!(ctx, "topic.node_removed")).into();
    };

    if !can_edit(&topic, uid, &store).await {
        return ctx.error(t!(ctx, "topic.no_edit_permission")).into();
    }

    match store
        .verify_token(uid, "edit-topic", form.token.as_str())
        .await
        .map_err(ctx.err())?
    {
        true => {}
        _ => return ctx.error(t!(ctx, "topic.form_expired_reopen")).into(),
    }

    let cfg = store.get_post_config().await;

    let BakeOutput {
        content_render,
        content_plain,
        at_users,
    } = match bake(&ctx, &cfg, Some(&form.title), &form.content) {
        Bake::Tips(msg) => {
            return EditPostTemplate {
                tips: vec![msg],
                token: store.get_token(uid, "edit-topic").await.unwrap_or_default(),
                topic: TopicDisplay {
                    title: form.title,
                    content: form.content,
                    ..topic
                },
                node,
                ctx: ctx.context(t_owned!(ctx, "topic.edit_topic")),
            }
            .into();
        }
        Bake::Output(data) => data,
    };

    notify.topic_at_users(id, at_users).await; // 给提及用户推送消息
    store
        .update_topic(id, form.title, form.content, content_render, content_plain)
        .await
        .map_err(ctx.err())?;
    moderator.push(Task::NewTopic(id)).await; // 推送消息到审核系统
    search.update_topic_idx(id).await; // 更新全文索引
    RedirectTemplate {
        tips: &t!(ctx, "topic.update_success"),
        url: &format!("/t/{}", id),
        ctx: ctx.context(t_owned!(ctx, "topic.update_success")),
    }
    .into()
}

async fn can_edit(topic: &TopicDisplay, uid: i64, store: &Store) -> bool {
    if topic.user_id != uid {
        return false;
    }
    if topic.is_locked {
        return false;
    }
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(topic.created_at);
    if duration.num_hours() >= 1 {
        return false;
    }
    let search = CommentSearch {
        topic: Some(topic.id),
        username: None,
        viewer_id: None,
    };
    let Ok(comments) = store.comments(&search, 1u32.into()).await else {
        return false;
    };
    if comments.total >= 10 {
        return false;
    }
    true
}

#[axum::debug_handler(state = AppState)]
pub async fn delete_confirm(ctx: Context, store: Store, Path(id): Path<i64>) -> WebResponse {
    let Some(article) = store.get_topic(id).await.map_err(ctx.err())? else {
        return ctx.error(t!(ctx, "topic.topic_not_found")).into();
    };

    let Some(uid) = ctx.get_uid() else {
        return LoginTipsTemplate {
            ctx: ctx.context(t_owned!(ctx, "topic.not_logged_in")),
        }
        .into();
    };

    if article.user_id != uid {
        return ctx.error(t!(ctx, "topic.no_delete_permission")).into();
    }

    let t = DeleteConfirmTemplate {
        topic: article,
        token: store
            .get_token(uid, "delete-topic")
            .await
            .unwrap_or_default(),
        ctx: ctx.context(t_owned!(ctx, "topic.delete_topic")),
    };
    t.into()
}

#[derive(Deserialize)]
pub struct DeletePostForm {
    token: String,
}

#[axum::debug_handler(state = AppState)]
pub async fn delete_post(
    ctx: Context,
    store: Store,
    Path(id): Path<i64>,
    GlobalContext(_notify): GlobalContext<NotifyDaemon>,
    GlobalContext(search): GlobalContext<SearchDaemon>,
    Form(form): Form<DeletePostForm>,
) -> WebResponse {
    let Some(article) = store.get_topic(id).await.map_err(ctx.err())? else {
        return ctx.error(t!(ctx, "topic.topic_not_found")).into();
    };

    let Some(user) = ctx.current_user.clone() else {
        return ctx.error(t!(ctx, "topic.please_login")).into();
    };

    if article.user_id != user.id {
        return ctx.error(t!(ctx, "topic.no_delete_permission")).into();
    }

    match store
        .verify_token(user.id, "delete-topic", form.token.as_str())
        .await
        .map_err(ctx.err())?
    {
        true => {}
        _ => return ctx.error(t!(ctx, "topic.submit_expired")).into(),
    }
    store.delete_topic(id).await.map_err(ctx.err())?;
    search.delete_topic_idx(id).await;
    RedirectTemplate {
        tips: &t!(ctx, "topic.delete_success"),
        url: "/",
        ctx: ctx.context(t_owned!(ctx, "topic.delete_success")),
    }
    .into()
}
pub mod manager {
    use super::DeletePostForm;
    use crate::common::{Context, CurrentUser, GlobalContext, WebResponse};
    use crate::daemon::notify::{Notify, NotifyDaemon};
    use crate::daemon::tantivy::SearchDaemon;
    use crate::home::topic::{DeleteTipsTemplate, LoginTipsTemplate};
    use crate::home::{AppState, RedirectTemplate};
    use crate::store::node::Node;
    use crate::store::topic::{CommentDisplay, TopicDisplay};
    use crate::store::Store;
    use crate::{t, t_owned};
    use askama::Template;
    use axum::extract::Path;
    use axum::response::IntoResponse;
    use axum::Form;
    use into_response_derive::TemplateResponse;
    use serde::Deserialize;

    #[derive(Template)]
    #[template(path = "home/topic/reply-delete-tips.html")]
    pub struct ReplyDeleteTipsTemplate<'a> {
        locale: &'a str,
        article: &'a TopicDisplay,
        user: &'a CurrentUser,
        content: &'a str,
        reason: &'a str,
    }
    #[derive(Template, TemplateResponse)]
    #[template(path = "home/topic/super/pin.html")]
    pub struct PinConfirmTemplate {
        topic: TopicDisplay,
        token: String,
        ctx: crate::common::VContext,
    }
    #[derive(Template)]
    #[template(path = "home/topic/move-tips.html")]
    pub struct MoveTipsTemplate<'a> {
        locale: &'a str,
        article: &'a TopicDisplay,
        node: &'a Node,
        user: &'a CurrentUser,
    }
    #[derive(Template, TemplateResponse)]
    #[template(path = "home/topic/super/move.html")]
    pub struct MoveConfirmTemplate {
        topic: TopicDisplay,
        nodes: Vec<Node>,
        token: String,
        ctx: crate::common::VContext,
    }

    #[derive(Deserialize)]
    pub struct MovePostForm {
        node_slug: String,
        token: String,
    }

    #[derive(Template, TemplateResponse)]
    #[template(path = "home/topic/super/delete.html")]
    pub struct AdminDeleteTopicTemplate {
        topic: TopicDisplay,
        token: String,
        ctx: crate::common::VContext,
    }

    #[derive(Template, TemplateResponse)]
    #[template(path = "home/topic/super/comment-delete.html")]
    pub struct AdminDeleteCommentTemplate {
        topic: TopicDisplay,
        comment: CommentDisplay,
        token: String,
        ctx: crate::common::VContext,
    }

    #[derive(Deserialize)]
    pub struct AdminDeleteForm {
        token: String,
        reason: String,
    }
    #[axum::debug_handler(state = AppState)]
    pub async fn move_confirm(ctx: Context, store: Store, Path(id): Path<i64>) -> WebResponse {
        let Some(topic) = store.get_topic(id).await.map_err(ctx.err())? else {
            return ctx.error(t!(ctx, "topic.topic_not_found")).into();
        };

        let Some(uid) = ctx.get_uid() else {
            return LoginTipsTemplate {
                ctx: ctx.context(t_owned!(ctx, "topic.not_logged_in")),
            }
            .into();
        };

        if !ctx
            .current_user
            .as_ref()
            .map(|u| u.is_moderator)
            .unwrap_or(false)
        {
            return ctx.error(t!(ctx, "topic.no_move_permission")).into();
        }

        let nodes = store.get_nodes().await.map_err(ctx.err())?;

        let t = MoveConfirmTemplate {
            topic,
            nodes,
            token: store.get_token(uid, "move-topic").await.unwrap_or_default(),
            ctx: ctx.context(t_owned!(ctx, "topic.move_topic")),
        };
        t.into()
    }

    #[axum::debug_handler(state = AppState)]
    pub async fn move_post(
        ctx: Context,
        store: Store,
        Path(id): Path<i64>,
        GlobalContext(notify): GlobalContext<NotifyDaemon>,
        Form(form): Form<MovePostForm>,
    ) -> WebResponse {
        let Some(article) = store.get_topic(id).await.map_err(ctx.err())? else {
            return ctx.error(t!(ctx, "topic.topic_not_found")).into();
        };
        let Some(node) = store.get_node(&form.node_slug).await.map_err(ctx.err())? else {
            return ctx.error(t!(ctx, "topic.node_not_found")).into();
        };
        let Some(user) = ctx.current_user.clone() else {
            return ctx.error(t!(ctx, "topic.please_login")).into();
        };
        if !user.is_moderator {
            return ctx.error(t!(ctx, "topic.no_permission")).into();
        }

        match store
            .verify_token(user.id, "move-topic", form.token.as_str())
            .await
            .map_err(ctx.err())?
        {
            true => {}
            _ => return ctx.error(t!(ctx, "topic.submit_expired")).into(),
        }

        store.move_topic(id, node.id).await.map_err(ctx.err())?;

        let article_uid = article.user_id;
        let article_id = article.id;
        let tips = MoveTipsTemplate {
            locale: "en_US", // TODO: 查询用户语言偏好
            article: &article,
            node: &node,
            user: &user,
        }
        .render()
        .unwrap_or_default();
        let msg = Notify::topic_move(article_uid, article_id, tips);
        notify.send(msg).await;
        RedirectTemplate {
            tips: &t!(ctx, "topic.operation_success"),
            url: &format!("/t/{}", id),
            ctx: ctx.context(t_owned!(ctx, "topic.operation_success")),
        }
        .into()
    }

    #[axum::debug_handler(state = AppState)]
    pub async fn delete_confirm(ctx: Context, store: Store, Path(id): Path<i64>) -> WebResponse {
        let Some(topic) = store.get_topic(id).await.map_err(ctx.err())? else {
            return ctx.error(t!(ctx, "topic.topic_not_found")).into();
        };

        let Some(uid) = ctx.get_uid() else {
            return LoginTipsTemplate {
                ctx: ctx.context(t_owned!(ctx, "topic.not_logged_in")),
            }
            .into();
        };

        if !ctx.is_moderator() {
            return ctx.error(t!(ctx, "topic.no_permission")).into();
        }

        let t = AdminDeleteTopicTemplate {
            topic,
            token: store
                .get_token(uid, "delete-topic-admin")
                .await
                .unwrap_or_default(),
            ctx: ctx.context(t_owned!(ctx, "topic.admin_delete_topic")),
        };
        t.into()
    }

    #[axum::debug_handler(state = AppState)]
    pub async fn delete_post(
        ctx: Context,
        store: Store,
        Path(id): Path<i64>,
        GlobalContext(notify): GlobalContext<NotifyDaemon>,
        GlobalContext(search): GlobalContext<SearchDaemon>,
        Form(form): Form<AdminDeleteForm>,
    ) -> WebResponse {
        let Some(article) = store.get_topic(id).await.map_err(ctx.err())? else {
            return ctx.error(t!(ctx, "topic.topic_not_found")).into();
        };

        let Some(user) = ctx.current_user.clone() else {
            return ctx.error(t!(ctx, "topic.please_login")).into();
        };

        if !user.is_moderator {
            return ctx.error(t!(ctx, "topic.no_permission")).into();
        }

        match store
            .verify_token(user.id, "delete-topic-admin", form.token.as_str())
            .await
            .map_err(ctx.err())?
        {
            true => {}
            _ => return ctx.error(t!(ctx, "topic.submit_expired")).into(),
        }

        let Some(trashed) = store.get_node("trashed").await.map_err(ctx.err())? else {
            return ctx.error(t!(ctx, "topic.delete_failed_trashed")).into();
        };

        let reason = if form.reason.trim().is_empty() {
            t_owned!(ctx, "topic.topic_violation")
        } else {
            form.reason
        };

        let tips = DeleteTipsTemplate {
            locale: "en_US", // TODO: 查询用户语言偏好
            article: &article,
            user: &user,
            reason,
        }
        .render()
        .unwrap_or_default();

        store.move_topic(id, trashed.id).await.map_err(ctx.err())?;

        let article_uid = article.user_id;
        let article_id = article.id;
        let msg = Notify::topic_move(article_uid, article_id, tips);
        notify.send(msg).await;
        search.delete_topic_idx(id).await; // 移除全文索引
        RedirectTemplate {
            tips: &t!(ctx, "topic.operation_success"),
            url: &format!("/go/{}", article.node_slug),
            ctx: ctx.context(t_owned!(ctx, "topic.operation_success")),
        }
        .into()
    }

    #[axum::debug_handler(state = AppState)]
    pub async fn comment_delete_confirm(
        ctx: Context,
        store: Store,
        Path(id): Path<i64>,
    ) -> WebResponse {
        let Some(uid) = ctx.get_uid() else {
            return ctx.error(t!(ctx, "topic.please_login")).into();
        };

        if !ctx.is_moderator() {
            return ctx.error(t!(ctx, "topic.no_permission")).into();
        }

        let Some(comment) = store.get_comment(id).await.map_err(ctx.err())? else {
            return ctx.error(t!(ctx, "topic.comment_not_found")).into();
        };

        let Some(topic) = store
            .get_topic(comment.article_id)
            .await
            .map_err(ctx.err())?
        else {
            return ctx.error(t!(ctx, "topic.topic_not_found")).into();
        };

        let t = AdminDeleteCommentTemplate {
            topic,
            comment,
            token: store
                .get_token(uid, "delete-comment-admin")
                .await
                .unwrap_or_default(),
            ctx: ctx.context(t_owned!(ctx, "topic.admin_delete_comment")),
        };
        t.into()
    }

    #[axum::debug_handler(state = AppState)]
    pub async fn comment_delete_post(
        ctx: Context,
        store: Store,
        current_user: CurrentUser,
        GlobalContext(notify): GlobalContext<NotifyDaemon>,
        Path(id): Path<i64>,
        Form(form): Form<AdminDeleteForm>,
    ) -> WebResponse {
        let Some(uid) = ctx.get_uid() else {
            return ctx.error(t!(ctx, "topic.please_login")).into();
        };

        if !ctx.is_moderator() {
            return ctx.error(t!(ctx, "topic.no_permission")).into();
        }

        let Some(comment) = store.get_comment(id).await.map_err(ctx.err())? else {
            return ctx.error(t!(ctx, "topic.comment_not_found")).into();
        };

        match store
            .verify_token(uid, "delete-comment-admin", form.token.as_str())
            .await
            .map_err(ctx.err())?
        {
            true => {}
            _ => return ctx.error(t!(ctx, "topic.submit_expired")).into(),
        }

        if let Some(article) = store
            .get_topic(comment.article_id)
            .await
            .map_err(ctx.err())?
        {
            let reason = if form.reason.trim().is_empty() {
                t_owned!(ctx, "topic.community_violation")
            } else {
                form.reason.clone()
            };

            let content = ReplyDeleteTipsTemplate {
                locale: "en_US", // TODO: 查询用户语言偏好
                article: &article,
                user: &current_user,
                content: &comment.content,
                reason: reason.as_ref(),
            }
            .render()
            .unwrap_or_default();
            notify
                .send(Notify::comment_delete(
                    comment.user_id,
                    comment.article_id,
                    content,
                ))
                .await;
        }

        store.delete_comment(id).await.map_err(ctx.err())?;
        RedirectTemplate {
            tips: &t!(ctx, "topic.comment_deleted"),
            url: &format!("/t/{}", comment.article_id),
            ctx: ctx.context(t_owned!(ctx, "topic.comment_deleted")),
        }
        .into()
    }

    #[axum::debug_handler(state = AppState)]
    pub async fn pin_confirm(ctx: Context, store: Store, Path(id): Path<i64>) -> WebResponse {
        let Some(topic) = store.get_topic(id).await.map_err(ctx.err())? else {
            return ctx.error(t!(ctx, "topic.topic_not_found")).into();
        };

        let Some(uid) = ctx.get_uid() else {
            return LoginTipsTemplate {
                ctx: ctx.context(t_owned!(ctx, "topic.not_logged_in")),
            }
            .into();
        };

        if !ctx
            .current_user
            .as_ref()
            .map(|u| u.is_moderator)
            .unwrap_or(false)
        {
            return ctx.error(t!(ctx, "topic.no_pin_permission")).into();
        }

        let t = PinConfirmTemplate {
            topic,
            token: store.get_token(uid, "pin-topic").await.unwrap_or_default(),
            ctx: ctx.context(t_owned!(ctx, "topic.pin_topic")),
        };
        t.into()
    }

    #[axum::debug_handler(state = AppState)]
    pub async fn pin_post(
        ctx: Context,
        store: Store,
        Path(id): Path<i64>,
        Form(form): Form<DeletePostForm>, // Reuse token form
    ) -> WebResponse {
        let Some(article) = store.get_topic(id).await.map_err(ctx.err())? else {
            return ctx.error(t!(ctx, "topic.topic_not_found")).into();
        };

        let Some(uid) = ctx.get_uid() else {
            return ctx.error(t!(ctx, "topic.please_login")).into();
        };

        if !ctx
            .current_user
            .as_ref()
            .map(|u| u.is_moderator)
            .unwrap_or(false)
        {
            return ctx.error(t!(ctx, "topic.no_permission")).into();
        }

        match store
            .verify_token(uid, "pin-topic", form.token.as_str())
            .await
            .map_err(ctx.err())?
        {
            true => {}
            _ => return ctx.error(t!(ctx, "topic.submit_expired")).into(),
        }

        store
            .pin_topic(id, !article.is_pinned)
            .await
            .map_err(ctx.err())?;
        RedirectTemplate {
            tips: &t!(ctx, "topic.operation_success"),
            url: &format!("/t/{}", id),
            ctx: ctx.context(t_owned!(ctx, "topic.operation_success")),
        }
        .into()
    }
}

#[axum::debug_handler(state = AppState)]
pub async fn lock_confirm(ctx: Context, store: Store, Path(id): Path<i64>) -> WebResponse {
    let Some(topic) = store.get_topic(id).await.map_err(ctx.err())? else {
        return ctx.error(t!(ctx, "topic.topic_not_found")).into();
    };

    let Some(uid) = ctx.get_uid() else {
        return LoginTipsTemplate {
            ctx: ctx.context(t_owned!(ctx, "topic.not_logged_in")),
        }
        .into();
    };
    if topic.is_locked && !ctx.is_moderator() {
        return ctx.error(t!(ctx, "topic.already_locked")).into();
    }

    if topic.user_id != uid
        && !ctx
            .current_user
            .as_ref()
            .map(|u| u.is_moderator)
            .unwrap_or(false)
    {
        return ctx.error(t!(ctx, "topic.no_lock_permission")).into();
    }

    let t = LockConfirmTemplate {
        topic,
        token: store.get_token(uid, "lock-topic").await.unwrap_or_default(),
        ctx: ctx.context(t_owned!(ctx, "topic.lock_topic")),
    };
    t.into()
}

#[axum::debug_handler(state = AppState)]
pub async fn lock_post(
    ctx: Context,
    store: Store,
    Path(id): Path<i64>,
    Form(form): Form<DeletePostForm>, // Reuse token form
) -> WebResponse {
    let Some(topic) = store.get_topic(id).await.map_err(ctx.err())? else {
        return ctx.error(t!(ctx, "topic.topic_not_found")).into();
    };

    let Some(uid) = ctx.get_uid() else {
        return ctx.error(t!(ctx, "topic.please_login")).into();
    };

    if topic.user_id != uid
        && !ctx
            .current_user
            .as_ref()
            .map(|u| u.is_moderator)
            .unwrap_or(false)
    {
        return ctx.error(t!(ctx, "topic.no_permission")).into();
    }

    match store
        .verify_token(uid, "lock-topic", form.token.as_str())
        .await
        .map_err(ctx.err())?
    {
        true => {}
        _ => return ctx.error(t!(ctx, "topic.submit_expired")).into(),
    }

    // 非管理员帖子锁定之后，不可以解锁
    if !ctx.is_moderator() && topic.is_locked {
        return ctx.error(t!(ctx, "topic.already_locked")).into();
    }

    store
        .lock_topic(id, !topic.is_locked)
        .await
        .map_err(ctx.err())?;
    RedirectTemplate {
        tips: &t!(ctx, "topic.operation_success"),
        url: &format!("/t/{}", id),
        ctx: ctx.context(t_owned!(ctx, "topic.operation_success")),
    }
    .into()
}

#[derive(Template)]
#[template(path = "home/topic/delete-tips.html")]
pub struct DeleteTipsTemplate<'a> {
    locale: &'a str,
    article: &'a TopicDisplay,
    user: &'a CurrentUser,
    reason: String,
}
#[derive(Template)]
#[template(path = "home/topic/reply-tips.html")]
pub struct ReplyTipsTemplate<'a> {
    locale: &'a str,
    article: &'a TopicDisplay,
    user: &'a CurrentUser,
}
pub fn plain(text: &[u8]) -> anyhow::Result<String> {
    Ok(html2text::config::with_decorator(TrivialDecorator::new()).string_from_read(text, 80)?)
}
#[derive(Deserialize)]
pub struct CommentForm {
    id: i64,
    content: String,
    token: String,
    #[serde(alias = "cf-turnstile-response")]
    pub turnstile_response: Option<String>,
}
#[axum::debug_handler(state = AppState)]
pub async fn comment_post(
    State(state): State<AppState>,
    ctx: Context,
    store: Store,
    GlobalContext(moderator): GlobalContext<ModerateDaemon>,
    GlobalContext(notify): GlobalContext<NotifyDaemon>,
    user: CurrentUser,
    Form(form): Form<CommentForm>,
) -> WebResponse {
    let Some(topic) = store.get_topic(form.id).await.map_err(ctx.err())? else {
        return Ok((
            StatusCode::NOT_FOUND,
            ctx.error(t!(ctx, "topic.reply_topic_not_found")),
        )
            .into_response());
    };

    if topic.is_locked {
        return ctx.error(t!(ctx, "topic.topic_locked_no_reply")).into();
    }
    if !store
        .verify_token(user.id, "comment", &form.token)
        .await
        .map_err(ctx.err())?
    {
        return ctx.error(t_owned!(ctx, "topic.form_expired")).into();
    }

    let turnstile_cfg = store.get_turnstile_config().await;
    let turnstile = turnstile_cfg.get_site_key("comment");
    if turnstile.is_some() {
        let Some(turnstile_response) = form.turnstile_response.clone().filter(|k| !k.is_empty())
        else {
            return ctx.error(t!(ctx, "auth.captcha_required")).into();
        };

        match turnstile_cfg.validate("comment", &turnstile_response).await {
            Ok(true) => {}
            Ok(false) => {
                return ctx.error(t!(ctx, "auth.captcha_failed")).into();
            }
            Err(err) => {
                return ctx.error(t!(ctx, "auth.captcha_error", err = err)).into();
            }
        }
    }

    let cfg = store.get_post_config().await;
    let is_author = user.id == topic.user_id;

    // 获取节点信息以检查奖励/消耗
    let Some(node) = store.get_node(&topic.node_slug).await.map_err(ctx.err())? else {
        return ctx.error(t!(ctx, "topic.node_not_found")).into();
    };
    if node.isolated && !is_author {
        return ctx.error("您没有权限回复这个帖子").into();
    }
    if node.access_only {
        return ctx.error(t!(ctx, "topic.node_readonly")).into();
    }

    let BakeOutput {
        content_render,
        content_plain,
        at_users,
    } = match bake(&ctx, &cfg, None, form.content.as_str()) {
        Bake::Tips(msg) => {
            return ctx.error(msg.as_str()).into();
        }
        Bake::Output(data) => data,
    };

    let user_detail = store.get_user_detail(user.id).await.map_err(ctx.err())?;
    if user_detail.credit_score <= 0 {
        return ctx.error(t!(ctx, "topic.account_abnormal_reply")).into();
    }
    // 检查金币是否足够
    if node.comment_reward < 0 {
        if user_detail.coins + node.comment_reward < 0 {
            return ctx.error(t!(ctx, "topic.coins_insufficient_reply")).into();
        }
    }

    let id = store
        .comment(
            user.id,
            form.id,
            form.content,
            content_render,
            content_plain,
        )
        .await
        .map_err(ctx.err())?;

    notify.comment_at_users(id, at_users).await;

    // 更新金币
    if node.comment_reward != 0 {
        if let Err(e) = store.update_coins(user.id, node.comment_reward).await {
            log::error!("Failed to update coins for user {}: {}", user.id, e);
        }
    }

    moderator.push(Task::NewComment(id)).await;
    if topic.user_id != user.id {
        let content = ReplyTipsTemplate {
            locale: "en_US", // TODO: 查询用户语言偏好
            article: &topic,
            user: &user,
        }
        .render()
        .unwrap_or_default();
        notify
            .send(Notify::reply(topic.user_id, topic.id, content))
            .await;
    }
    // 回复之后跳转到帖子回复最后一页
    let count = store
        .comments_count(topic.id, Some(user.id))
        .await
        .inspect_err(|err| warn!("failed to query the number of replies to the post: {}", err))
        .unwrap_or((topic.reply_count + 1) as u32);
    let comments_per_page = state.cfg.load().visit.comments_per_page; // 加载全局配置
    let page = if count % comments_per_page == 0 {
        count / comments_per_page
    } else {
        count / comments_per_page + 1
    };

    // 以后再添加一个配置，是否展示一次跳转页面
    if (1 + 1) < 0 {
        Ok(ctx
            .redirect("回复成功，正在返回", &format!("/t/{}?p={}", topic.id, page))
            .into_response())
    } else {
        Ok(Redirect::to(&format!("/t/{}?p={}", topic.id, page)).into_response())
    }
}

#[axum::debug_handler(state = AppState)]
pub async fn comment_delete(
    ctx: Context,
    store: Store,
    GlobalContext(_notify): GlobalContext<NotifyDaemon>,
    Path(id): Path<i64>,
) -> WebResponse {
    let Some(uid) = ctx.get_uid() else {
        return ctx.error(t!(ctx, "topic.no_action_permission")).into();
    };

    let Some(comment) = store.get_comment(id).await.map_err(ctx.err())? else {
        return ctx.error(t!(ctx, "topic.comment_not_found")).into();
    };

    if comment.user_id != uid {
        return ctx.error(t!(ctx, "topic.no_action_permission")).into();
    };

    store.delete_comment(id).await.map_err(ctx.err())?;
    RedirectTemplate {
        tips: &t!(ctx, "topic.comment_deleted"),
        url: &format!("/t/{}", comment.article_id),
        ctx: ctx.context(t_owned!(ctx, "topic.comment_deleted")),
    }
    .into()
}
#[axum::debug_handler(state = AppState)]
pub async fn read(
    ctx: Context,
    store: Store,
    State(_state): State<AppState>,
    GlobalContext(link_filter): GlobalContext<LinkFilter>,
    Path(id): Path<i64>,
    page: Page,
    user: OptionalCurrentUser,
) -> WebResponse {
    let Some(mut topic) = store.get_topic(id).await.map_err(ctx.err())? else {
        return Ok((
            StatusCode::NOT_FOUND,
            ctx.error(t!(ctx, "topic.topic_not_found")),
        )
            .into_response());
    };
    let Some(node) = store.get_node(&topic.node_slug).await.map_err(ctx.err())? else {
        return ctx.error(t!(ctx, "topic.node_info_not_found")).into();
    };

    let uid = user.as_ref().map(|u| u.id.clone());
    let is_author = user.as_ref().map(|u| u.id == topic.user_id).unwrap_or(false);
    let is_moderator = ctx.is_moderator();

    // TODO: 添加一个功能，只有作者 At 到的用户才可以查看
    if node.isolated && !is_author {
        return ctx.error("您没有权限查看这个帖子").into();
    }
    if node.moderator_access_required && !is_moderator {
        return ctx.error(t!(ctx, "topic.mod_access_only")).into();
    }
    if node.member_access_required && user.is_none() {
        return ctx.error(t!(ctx, "topic.member_access_only")).into();
    }

    if uid == Some(topic.user_id) {
        if can_edit(&topic, uid.unwrap_or_default(), &store).await {
            topic.ops.push("edit".to_string());
        }
        topic.ops.push("delete".to_string());
        if !topic.is_locked || is_moderator {
            topic.ops.push("lock".to_string());
        }
    } else if is_moderator {
        topic.ops.push("super-delete".to_string());
        topic.ops.push("lock".to_string());
    }

    if is_moderator {
        topic.ops.push("pin".to_string());
    }

    let _ = store.incr_view_count(id).await;

    let search = &CommentSearch {
        topic: Some(topic.id),
        username: None,
        viewer_id: uid,
    };
    let mut comments = store
        .comments(&search, page.comment())
        .await
        .map_err(ctx.err())?;

    for comment in comments.data.iter_mut() {
        if uid == Some(comment.user_id) {
            comment.ops.push("delete".to_string());
        } else if is_moderator {
            comment.ops.push("super-delete".to_string());
        }

        // 动态过滤回复链接，这里出现 String 拷贝是否有更加高效的写法？
        if let Ok(new_content) = link_filter.filter(comment.content_render.clone()).await {
            comment.content_render = new_content;
        };
    }

    // 动态过滤帖子链接
    if let Ok(new_content) = link_filter.filter(topic.content_render.clone()).await {
        topic.content_render = new_content;
    };

    let turnstile_cfg = store.get_turnstile_config().await;
    let turnstile = turnstile_cfg.get_site_key("comment");

    let title = topic.title.clone();
    let token = if let Some(user) = user.as_ref() {
        store
            .get_token(user.id, "comment")
            .await
            .unwrap_or_default()
    } else {
        "".to_string()
    };
    TopicDetailTemplate {
        ctx: ctx.context(title),
        topic,
        node,
        comments,
        current_user: user,
        turnstile,
        token,
    }
    .into()
}

#[derive(Template, TemplateResponse)]
#[template(path = "home/recent.html")]
struct RecentTemplate {
    articles: Pagination<TopicIndex>,
    ctx: crate::common::VContext,
}

#[axum::debug_handler(state = AppState)]
pub async fn rss_recent(
    ctx: Context,
    store: Store,
    page: Page,
    user: OptionalCurrentUser,
) -> impl IntoResponse {
    let search = TopicSearch {
        sort: None,
        q: None,
        user_id: None,
        username: None,
        node_id: None,
        node_slug: None,
        viewer_id: user.map(|u| u.id),
        recent_flag: true,
    };
    let articles = match store.topics(&search, page.topic()).await {
        Ok(data) => data,
        Err(err) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response();
        }
    };

    let domain = if ctx.website.domain.is_empty() {
        "http://localhost:3000".to_string()
    } else if ctx.website.domain.starts_with("http") {
        ctx.website.domain.clone()
    } else {
        format!("https://{}", ctx.website.domain)
    };

    let items = articles
        .data
        .into_iter()
        .map(|article| {
            let link = format!("{}/t/{}", domain, article.id);
            let description = "".to_string();
            let pub_date = article.created_at.to_rfc2822();
            ItemBuilder::default()
                .title(Some(article.title))
                .link(Some(link.clone()))
                .description(Some(description))
                .author(Some(article.username))
                .pub_date(Some(pub_date))
                .guid(Some(Guid {
                    value: link,
                    permalink: true,
                }))
                .build()
        })
        .collect::<Vec<_>>();

    let channel = ChannelBuilder::default()
        .title(format!(
            "{} - {}",
            t!(ctx, "topic.recent_topics"),
            ctx.website.name
        ))
        .link(format!("{}/recent", domain))
        .description(ctx.website.description)
        .items(items)
        .build();

    (
        [(
            axum::http::header::CONTENT_TYPE,
            "application/rss+xml; charset=utf-8",
        )],
        channel.to_string(),
    )
        .into_response()
}
#[axum::debug_handler(state = AppState)]
pub async fn recent(
    ctx: Context,
    store: Store,
    page: Page,
    user: OptionalCurrentUser,
) -> WebResponse {
    let search = TopicSearch {
        sort: Some("rank".to_string()),
        q: None,
        user_id: None,
        username: None,
        node_id: None,
        node_slug: None,
        viewer_id: user.map(|u| u.id),
        recent_flag: true,
    };
    let articles = store
        .topics(&search, page.topic())
        .await
        .map_err(ctx.err())?;

    let t = RecentTemplate {
        articles,
        ctx: ctx.context(t_owned!(ctx, "topic.recent_topics")),
    };
    t.into()
}
