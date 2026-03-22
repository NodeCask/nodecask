use crate::store::Store;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Node {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub show_in_list: bool,
    pub background_image: String,
    pub icon_image: String,
    pub node_color: String,
    pub custom_html: String,
    pub member_access_required: bool,
    pub moderator_access_required: bool,
    pub isolated: bool,
    pub access_only: bool,
    pub topic_reward: i64,
    pub comment_reward: i64,
    pub topic_count: i64,
    pub created_at: DateTime<Utc>,
}
impl Node {
    pub fn url(&self) -> String {
        format!("/go/{}", self.slug)
    }
}
impl Store {
    pub async fn get_node(&self, slug: &str) -> sqlx::Result<Option<Node>> {
        sqlx::query_as::<_, Node>(
            r#"SELECT id,
       name,
       slug,
       description,
       created_at,
       show_in_list,
       background_image,
       icon_image,
       node_color,
       custom_html,
       member_access_required,
       moderator_access_required,
       topic_reward,
       comment_reward,
       isolated,
       access_only,
       topic_count
FROM node
WHERE slug = ?
limit 1"#,
        )
            .bind(slug)
            .fetch_optional(&self.pool)
            .await
    }
    pub async fn get_node_by_id(&self, id: i64) -> sqlx::Result<Option<Node>> {
        sqlx::query_as::<_, Node>(
            r#"SELECT id,
       name,
       slug,
       description,
       created_at,
       show_in_list,
       background_image,
       icon_image,
       node_color,
       custom_html,
       member_access_required,
       moderator_access_required,
       topic_reward,
       comment_reward,
       isolated,
       access_only,
       topic_count
FROM node
WHERE id = ?"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn get_nodes(&self) -> sqlx::Result<Vec<Node>> {
        sqlx::query_as::<_, Node>("SELECT * FROM node ORDER BY created_at")
            .fetch_all(&self.pool)
            .await
    }

    pub async fn exist_node_slug(&self, slug: &str) -> sqlx::Result<bool> {
        let row: bool = sqlx::query_scalar("select count(*) from node where slug = ?")
            .bind(slug)
            .fetch_one(&self.pool)
            .await?;
        Ok(row)
    }

    pub async fn create_node(
        &self,
        name: &str,
        slug: &str,
        description: &str,
        show_in_list: bool,
        background_image: &str,
        icon_image: &str,
        node_color: &str,
        custom_html: &str,
        member_access_required: bool,
        moderator_access_required: bool,
        isolated: bool,
        access_only: bool,
        topic_reward: i64,
        comment_reward: i64,
    ) -> sqlx::Result<()> {
        sqlx::query(
            r#"INSERT INTO node (name, slug, description,
                   show_in_list, background_image, icon_image,
                   node_color, custom_html, member_access_required, moderator_access_required, isolated, access_only, topic_reward, comment_reward, created_at)
VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))"#,
        )
        .bind(name)
        .bind(slug)
        .bind(description)
        .bind(show_in_list)
        .bind(background_image)
        .bind(icon_image)
        .bind(node_color)
        .bind(custom_html)
        .bind(member_access_required)
        .bind(moderator_access_required)
        .bind(isolated)
        .bind(access_only)
        .bind(topic_reward)
        .bind(comment_reward)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_node(
        &self,
        id: i64,
        name: &str,
        slug: &str,
        description: &str,
        show_in_list: bool,
        background_image: &str,
        icon_image: &str,
        node_color: &str,
        custom_html: &str,
        member_access_required: bool,
        moderator_access_required: bool,
        isolated: bool,
        access_only: bool,
        topic_reward: i64,
        comment_reward: i64,
    ) -> sqlx::Result<bool> {
        let result = sqlx::query(
            r#"update node
set name             = ?,
    slug             = ?,
    description      = ?,
    show_in_list     = ?,
    background_image = ?,
    icon_image       = ?,
    node_color       = ?,
    custom_html      = ?,
    member_access_required = ?,
    moderator_access_required = ?,
    isolated = ?,
    access_only = ?,
    topic_reward = ?,
    comment_reward = ?
where id = ?
    "#,
        )
        .bind(name)
        .bind(slug)
        .bind(description)
        .bind(show_in_list)
        .bind(background_image)
        .bind(icon_image)
        .bind(node_color)
        .bind(custom_html)
        .bind(member_access_required)
        .bind(moderator_access_required)
        .bind(isolated)
        .bind(access_only)
        .bind(topic_reward)
        .bind(comment_reward)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // 删除节点的时候，必须提供一个新的节点 ID 用来防止关联帖子给删除
    pub async fn delete_node(&self, id: i64, move_to: i64) -> sqlx::Result<bool> {
        let mut tx = self.pool.begin().await?;
        let _ = sqlx::query("update topic set node_id = ? where node_id = ? ")
            .bind(move_to)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        let result = sqlx::query("DELETE FROM node WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(result.rows_affected() > 0)
    }
}