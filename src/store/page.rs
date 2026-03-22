use crate::store::Store;
use chrono::{DateTime, Utc};
use pulldown_cmark::Parser;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct CustomPage {
    pub id: i64,
    pub path: String,
    pub title: String,
    pub description: String,
    pub content_type: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CustomPage {
    pub fn get_render_content(&self) -> String {
        if self.content_type == "markdown" {
            let mut html_output = String::new();
            pulldown_cmark::html::push_html(&mut html_output, Parser::new(&self.content));
            html_output
        } else {
            self.content.clone()
        }
    }
}

impl Store {
    pub async fn get_page(&self, path: &str) -> Option<CustomPage> {
        sqlx::query_as::<_, CustomPage>(
            r#"SELECT id, path, title, description, content_type, content, created_at, updated_at
            FROM page WHERE path = ?"#,
        )
        .bind(path)
        .fetch_optional(&self.pool)
        .await
        .unwrap_or_default()
    }

    pub async fn get_pages(&self) -> Vec<CustomPage> {
        sqlx::query_as::<_, CustomPage>(
            r#"SELECT id, path, title, description, content_type, content, created_at, updated_at
            FROM page ORDER BY created_at DESC"#
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default()
    }

    pub async fn create_page(
        &self,
        path: &str,
        title: &str,
        description: &str,
        content_type: &str,
        content: &str,
    ) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            r#"INSERT INTO page (path, title, description, content_type, content)
            VALUES (?, ?, ?, ?, ?)"#,
        )
        .bind(path)
        .bind(title)
        .bind(description)
        .bind(content_type)
        .bind(content)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn update_page(
        &self,
        id: i64,
        path: &str,
        title: &str,
        description: &str,
        content_type: &str,
        content: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE page SET path = ?, title = ?, description = ?, content_type = ?, content = ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?"#,
        )
        .bind(path)
        .bind(title)
        .bind(description)
        .bind(content_type)
        .bind(content)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_page(&self, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM page WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
