use crate::common::{Context, OptionalCurrentUser, WebResponse};
use crate::common::AppState;
use crate::store::node::Node;
use crate::store::topic::{TopicIndex, TopicSearch};
use crate::store::{Page, Pagination, Store};
use crate::{t, t_owned};
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use into_response_derive::TemplateResponse;
use rss::{ChannelBuilder, Guid, ItemBuilder};

#[derive(Template, TemplateResponse)]
#[template(path = "home/node/detail.html")]
struct NodeTemplate {
    node: Node,
    articles: Pagination<TopicIndex>,
    ctx: crate::common::VContext,
}

#[derive(Template, TemplateResponse)]
#[template(path = "home/node/list.html")]
struct NodesTemplate {
    nodes: Vec<Node>,
    ctx: crate::common::VContext,
}
#[axum::debug_handler(state = AppState)]
pub async fn rss(
    ctx: Context,
    store: Store,
    Path(slug): Path<String>,
    page: Page,
    user: OptionalCurrentUser,
) -> impl IntoResponse {
    let Ok(Some(node)) = store.get_node(&slug).await else {
        return (StatusCode::NOT_FOUND, t!(ctx, "node.not_found")).into_response();
    };

    // Check permissions
    if node.moderator_access_required {
        let is_mod = user.as_ref().map(|u| u.is_moderator).unwrap_or(false);
        if !is_mod {
            return (StatusCode::FORBIDDEN, t!(ctx, "node.moderator_only")).into_response();
        }
    }
    if node.member_access_required {
        if user.is_none() {
            return (StatusCode::FORBIDDEN, t!(ctx, "node.member_only")).into_response();
        }
    }

    let search = TopicSearch {
        node_id: Some(node.id),
        viewer_id: user.as_ref().map(|u| u.id),
        ..Default::default()
    };
    let articles = match store.topics(&search, page.topic()).await {
        Ok(articles) => articles,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("{}: {}", t!(ctx, "node.fetch_topics_error"), err),
            )
                .into_response();
        }
    };

    let domain = if ctx.website.domain.is_empty() {
        "http://localhost:3000".to_string()
    } else if ctx.website.domain.starts_with("http") {
        ctx.website.domain.clone()
    } else {
        format!("https://{}", ctx.website.domain)
    };

    let mut items = Vec::new();
    for article in articles.data {
        let link = format!("{}/t/{}", domain, article.id);
        let description = "".to_string(); // TODO: 添加第一段
        let pub_date = article.created_at.to_rfc2822();

        let item = ItemBuilder::default()
            .title(Some(article.title))
            .link(Some(link.clone()))
            .description(Some(description))
            .author(Some(article.username))
            .pub_date(Some(pub_date))
            .guid(Some(Guid {
                value: link,
                permalink: true,
            }))
            .build();
        items.push(item);
    }

    let channel = ChannelBuilder::default()
        .title(format!("{} - {}", node.name, ctx.website.name))
        .link(format!("{}/go/{}", domain, node.slug))
        .description(node.description)
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
pub async fn topics(
    ctx: Context,
    store: Store,
    State(_state): State<AppState>,
    Path(slug): Path<String>,
    page: Page,
    user: OptionalCurrentUser,
) -> WebResponse {
    let Some(node) = store.get_node(&slug).await.map_err(ctx.err())? else {
        return Ok((StatusCode::NOT_FOUND, ctx.error(t!(ctx, "node.not_found"))).into_response());
    };

    // Check permissions
    if node.moderator_access_required {
        let is_mod = user.as_ref().map(|u| u.is_moderator).unwrap_or(false);
        if !is_mod {
            return ctx.error(t!(ctx, "node.moderator_only")).into();
        }
    }
    if node.member_access_required {
        if user.is_none() {
            return ctx.error(t!(ctx, "node.member_only")).into();
        }
    }

    let search = TopicSearch {
        node_id: Some(node.id),
        viewer_id: user.as_ref().map(|u| u.id),
        ..Default::default()
    };
    let articles = store.topics(&search, page.topic()).await.map_err(ctx.err())?;

    let t = NodeTemplate {
        ctx: ctx.context(node.name.clone()),
        node,
        articles,
    };
    t.into()
}

#[axum::debug_handler(state = AppState)]
pub async fn nodes(
    ctx: Context,
    store: Store,
    State(_state): State<AppState>,
    _user: OptionalCurrentUser,
) -> WebResponse {
    let nodes = store
        .get_nodes()
        .await
        .map_err(ctx.err())?
        .into_iter()
        .filter(|node| node.show_in_list)
        .collect::<Vec<Node>>();
    NodesTemplate {
        nodes,
        ctx: ctx.context(t_owned!(ctx, "node.list")),
    }
    .into()
}

#[derive(Template, TemplateResponse)]
#[template(path = "home/node/list-new.html")]
struct NodesNewTemplate {
    nodes: Vec<Node>,
    ctx: crate::common::VContext,
}

#[axum::debug_handler(state = AppState)]
pub async fn nodes_new(
    ctx: Context,
    store: Store,
    State(_state): State<AppState>,
    _user: OptionalCurrentUser,
) -> WebResponse {
    let nodes = store
        .get_nodes()
        .await
        .map_err(ctx.err())?
        .into_iter()
        .filter(|n| n.show_in_list)
        .collect();
    NodesNewTemplate {
        nodes,
        ctx: ctx.context(t_owned!(ctx, "node.choose_post")),
    }
    .into()
}

#[derive(Template, TemplateResponse)]
#[template(path = "home/statistics.html")]
struct StatisticsTemplate {
    user_count: i64,
    topic_count: i64,
    articles: Vec<TopicIndex>,
    ctx: crate::common::VContext,
}

#[axum::debug_handler(state = AppState)]
pub async fn statistics(
    ctx: Context,
    store: Store,
    State(_state): State<AppState>,
    user: OptionalCurrentUser,
) -> WebResponse {
    let user_count = store.get_user_count().await.unwrap_or(0);
    let topic_count = store.get_topic_count().await.unwrap_or(0);
    let search = TopicSearch {
        sort: Some("latest".to_string()),
        viewer_id: user.as_ref().map(|u| u.id),
        ..Default::default()
    };
    let articles = store
        .topics(&search, 1.into())
        .await
        .map_err(ctx.err())?
        .data;

    StatisticsTemplate {
        user_count,
        topic_count,
        articles,
        ctx: ctx.context(t_owned!(ctx, "node.statistics")),
    }
    .into()
}
