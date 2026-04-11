use crate::common::AppState;
use crate::moderator::{Data, ModContext as Context};
use crate::store::Store;
use axum::Json;
use axum::extract::Path;
use axum::response::IntoResponse;
use serde::Deserialize;

const PROTECTED_NODES: &[&'static str] = &["trashed", "unclassified", "unclassified-protect"];
#[derive(Deserialize)]
pub struct NodeUpdate {
    pub name: String,
    pub slug: String,
    pub description: String,
    pub show_in_list: bool,
    pub member_access_required: bool,
    pub moderator_access_required: bool,
    pub isolated: bool,
    pub access_only: bool,
    pub topic_reward: i64,
    pub comment_reward: i64,
}

#[axum::debug_handler(state = AppState)]
pub async fn list(
    _ctx: Context,
    store: Store,
) -> impl IntoResponse {
    let nodes = store.get_nodes().await.unwrap_or_default();
    Data::ok(nodes).into_response()
}

#[axum::debug_handler(state = AppState)]
pub async fn create(
    _ctx: Context,
    store: Store,
    Json(data): Json<NodeUpdate>,
) -> impl IntoResponse {
    if data.name.is_empty() {
        return Data::fail("节点名称不能为空").into_response();
    }
    if data.slug.is_empty() {
        return Data::fail("节点 Slug 不能为空").into_response();
    }

    if store.exist_node_slug(&data.slug).await.unwrap_or_default() {
        return Data::<()>::error(&format!("slug ({}) 已经存在", &data.slug)).into_response();
    }

    let result = store.create_node(
        &data.name,
        &data.slug,
        &data.description,
        data.show_in_list,
        data.member_access_required,
        data.moderator_access_required,
        data.isolated,
        data.access_only,
        data.topic_reward,
        data.comment_reward,
    ).await;

    match result {
        Ok(_) => Data::done().into_response(),
        Err(e) => Data::<()>::error(&format!("创建失败: {}", e)).into_response(),
    }
}

#[axum::debug_handler(state = AppState)]
pub async fn update(
    _ctx: Context,
    store: Store,
    Path(id): Path<i64>,
    Json(data): Json<NodeUpdate>,
) -> impl IntoResponse {
    if data.name.is_empty() {
        return Data::fail("节点名称不能为空").into_response();
    }
    if data.slug.is_empty() {
        return Data::fail("节点 Slug 不能为空").into_response();
    }
    if !is_name_valid(&data.slug) {
        return Data::fail("节点 Slug 只能字母、数字、横杆").into_response();
    }
    if data.slug.len() > 100 {
        return Data::fail("节点 Slug 最长 100 个字符").into_response();
    }

    match store.get_node(&data.slug).await {
        Ok(Some(node)) => {
            if node.id != id {
                return Data::fail("节点 Slug 已经存在").into_response();
            }
            if node.id == id && PROTECTED_NODES.contains(&node.slug.as_str()) {
                return Data::fail("内置节点不允许修改 Slug 参数").into_response();
            }
        },
        Ok(None) => {}
        Err(err) => {
            return Data::fail(&format!("数据库读写异常: {}", err)).into_response();
        }
    }
    let result = store.update_node(
        id,
        &data.name,
        &data.slug,
        &data.description,
        data.show_in_list,
        data.member_access_required,
        data.moderator_access_required,
        data.isolated,
        data.access_only,
        data.topic_reward,
        data.comment_reward,
    ).await;

    match result {
        Ok(found) => {
            if !found {
                Data::fail("节点未找到").into_response()
            } else {
                Data::done().into_response()
            }
        }
        Err(e) => Data::<()>::error(&format!("更新失败: {}", e)).into_response(),
    }
}

fn is_name_valid(name: &str) -> bool {
    !name.is_empty() && name.split('-').all(|part| {
        !part.is_empty() && part.chars().all(|c|c.is_ascii_alphanumeric())
    })
}
#[axum::debug_handler(state = AppState)]
pub async fn delete(
    _ctx: Context,
    store: Store,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let node = match store.get_node_by_id(id).await {
        Ok(Some(n)) => n,
        Ok(None) => {
            return Data::fail("节点未找到或是已经删除").into_response();
        },
        Err(err) => {
            return Data::fail(&format!("查询节点信息错误: {}", err)).into_response();
        }
    };
    if PROTECTED_NODES.contains(&node.slug.as_str()) {
        return Data::fail("内置节点无法删除").into_response();
    }
    
    let unclassified = match store.get_node("unclassified").await {
        Ok(Some(node)) => node.id,
        Ok(None) => { id }
        Err(err) => {
            return Data::fail(&format!("查询节点错误: {}", err)).into_response();
        }
    };

    let result = store.delete_node(id, unclassified).await;

    match result {
        Ok(found) => {
            if !found {
                Data::<()>::error("节点未找到").into_response()
            } else {
                Data::done().into_response()
            }
        }
        Err(e) => Data::<()>::error(&format!("删除失败: {}", e)).into_response(),
    }
}

#[derive(Deserialize)]
pub struct NodeAttrUpdate {
    pub key: String,
    pub value: String,
}
#[derive(Deserialize)]
pub struct NodeAttrDelete {
    pub key: String,
}

#[axum::debug_handler(state = AppState)]
pub async fn get_attributes(
    _ctx: Context,
    store: Store,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match store.get_node_by_id(id).await {
        Ok(Some(node)) => {
            Data::ok(node.attributes).into_response()
        }
        Ok(None) => {
            Data::fail("节点未找到").into_response()
        }
        Err(err) => {
            Data::fail(&format!("数据库读写异常: {}", err)).into_response()
        }
    }
}

#[axum::debug_handler(state = AppState)]
pub async fn update_attributes(
    _ctx: Context,
    store: Store,
    Path(id): Path<i64>,
    Json(data): Json<NodeAttrUpdate>,
) -> impl IntoResponse {
    if data.key.is_empty() {
        return Data::fail("属性名不能为空").into_response();
    }

    match store.get_node_by_id(id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            return Data::fail("节点未找到").into_response();
        }
        Err(err) => {
            return Data::fail(&format!("数据库读写异常: {}", err)).into_response();
        }
    }

    let result = store.update_node_attr(id, &data.key, Some(&data.value)).await;

    match result {
        Ok(found) => {
            if !found {
                Data::fail("节点未找到").into_response()
            } else {
                Data::done().into_response()
            }
        }
        Err(e) => Data::<()>::error(&format!("更新属性失败: {}", e)).into_response(),
    }
}

#[axum::debug_handler(state = AppState)]
pub async fn remove_attributes(
    _ctx: Context,
    store: Store,
    Path(id): Path<i64>,
    Json(data): Json<NodeAttrDelete>,
) -> impl IntoResponse {
    if data.key.is_empty() {
        return Data::fail("属性名不能为空").into_response();
    }

    match store.get_node_by_id(id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            return Data::fail("节点未找到").into_response();
        }
        Err(err) => {
            return Data::fail(&format!("数据库读写异常: {}", err)).into_response();
        }
    }

    let result = store.update_node_attr(id, &data.key, None).await;

    match result {
        Ok(found) => {
            if !found {
                Data::fail("节点未找到").into_response()
            } else {
                Data::done().into_response()
            }
        }
        Err(e) => Data::<()>::error(&format!("更新属性失败: {}", e)).into_response(),
    }
}
