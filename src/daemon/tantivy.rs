use crate::store::topic::TopicDisplay;
use crate::store::Store;
use anyhow::anyhow;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{
    Field, IndexRecordOption, NumericOptions, Schema, TextFieldIndexing, TextOptions, Value, STORED,
    TEXT,
};
use tantivy::{doc, DocAddress, Index, IndexWriter, Score, TantivyDocument, Term};
use tokio::sync::mpsc::Sender;

#[derive(Debug, Clone, Copy)]
struct TopicFieldTable {
    id: Field,
    title: Field,
    content: Field,
    username: Field,
    node: Field,
    node_slug: Field,
    date: Field,
}
impl TryFrom<&Schema> for TopicFieldTable {
    type Error = anyhow::Error;

    fn try_from(schema: &Schema) -> Result<Self, Self::Error> {
        let id = schema.get_field("id")?;
        let title = schema.get_field("title")?;
        let content = schema.get_field("content")?;
        let username = schema.get_field("username")?;
        let date = schema.get_field("date")?;
        let node = schema.get_field("node")?;
        let node_slug = schema.get_field("node_slug")?;
        Ok(Self {
            id,
            title,
            content,
            username,
            node,
            node_slug,
            date,
        })
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TopicItem {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub username: String,
    pub node: String,
    pub node_slug: String,
    pub date: chrono::DateTime<chrono::Utc>,
}
impl TryFrom<(&TopicFieldTable, TantivyDocument)> for TopicItem {
    type Error = anyhow::Error;

    fn try_from((t, doc): (&TopicFieldTable, TantivyDocument)) -> Result<Self, Self::Error> {
        let Some(id) = doc.get_first(t.id).and_then(|v| v.as_i64()) else {
            return Err(anyhow!("id 未找到"));
        };
        let Some(title) = doc
            .get_first(t.title)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
        else {
            return Err(anyhow!("title 未找到"));
        };
        let Some(content) = doc
            .get_first(t.content)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
        else {
            return Err(anyhow!("content 未找到"));
        };
        let Some(username) = doc
            .get_first(t.username)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
        else {
            return Err(anyhow!("username 未找到"));
        };
        let Some(node) = doc
            .get_first(t.node)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
        else {
            return Err(anyhow!("node 未找到"));
        };
        let Some(node_slug) = doc
            .get_first(t.node_slug)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
        else {
            return Err(anyhow!("node_slug 未找到"));
        };
        let Some(date) = doc
            .get_first(t.date)
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&chrono::Utc))
        else {
            return Err(anyhow!("node_slug 未找到"));
        };
        Ok(Self {
            id,
            title,
            content,
            username,
            node,
            node_slug,
            date,
        })
    }
}

type Message = tokio::sync::oneshot::Sender<anyhow::Result<Vec<TopicItem>>>;
enum Instruct {
    TopicUpdate(i64),
    TopicDelete(i64),
    Search(String, Message),
}

#[derive(Clone)]
pub(crate) struct SearchDaemon {
    channel: Sender<Instruct>,
}

impl SearchDaemon {
    pub fn new(store: Store, path: impl AsRef<Path>) -> anyhow::Result<SearchDaemon> {
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<Instruct>(100);
        let idx = IndexService::new(path)?;
        tokio::spawn(async move {
            while let Some(ins) = receiver.recv().await {
                match ins {
                    Instruct::TopicUpdate(id) => {
                        let Ok(Some(topic)) = store.get_topic(id).await.inspect_err(|err| {
                            warn!("Failed to query topic info: {}", err);
                        }) else {
                            continue;
                        };
                        if let Err(err) = idx.make_idx_topic(topic) {
                            warn!("Failed to build topic fulltext index: {}", err);
                        }
                    }
                    Instruct::Search(keyword, sender) => {
                        if let Err(_err) = sender.send(idx.topic_search(&keyword)) {
                            warn!("Unexpected error sending search result");
                        }
                    }
                    Instruct::TopicDelete(id) => {
                        if let Err(err) = idx.pure_idx_topic(id) {
                            warn!("Failed to delete topic index: {}", err);
                        }
                    }
                }
            }
            info!("SearchDaemon exiting...");
        });
        Ok(SearchDaemon { channel: sender })
    }
    pub async fn search(&self, keyword: String) -> anyhow::Result<Vec<TopicItem>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.channel.send(Instruct::Search(keyword, tx)).await?;
        rx.await?
    }
    pub async fn update_topic_idx(&self, id: i64) {
        if let Err(err) = self.channel.send(Instruct::TopicUpdate(id)).await {
            warn!("Failed to update topic index: {}", err);
        }
    }
    pub async fn delete_topic_idx(&self, id: i64) {
        if let Err(err) = self.channel.send(Instruct::TopicDelete(id)).await {
            warn!("Failed to delete topic index: {}", err);
        }
    }
}

/// IndexService 是一个同步服务，通过 SearchDaemon 转换成为异步服务
#[derive(Clone, Debug)]
pub struct IndexService {
    topic: tantivy::Index,
    topic_fields: TopicFieldTable,
}

impl IndexService {
    pub fn new(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let topic = if !path.as_ref().join("topic").exists() {
            let _ = std::fs::create_dir_all(path.as_ref().join("topic"));
            let opts = TextOptions::default()
                .set_indexing_options(
                    TextFieldIndexing::default()
                        .set_tokenizer("jieba")
                        .set_index_option(IndexRecordOption::WithFreqsAndPositions),
                )
                .set_stored();
            let mut schema_builder = Schema::builder();
            schema_builder
                .add_i64_field("id", NumericOptions::default().set_stored().set_indexed());
            schema_builder.add_text_field("title", opts.clone());
            schema_builder.add_text_field("content", opts);
            schema_builder.add_text_field("date", TEXT | STORED);
            schema_builder.add_text_field("username", STORED);
            schema_builder.add_text_field("node", STORED);
            schema_builder.add_text_field("node_slug", STORED);
            let schema = schema_builder.build();
            Index::create_in_dir(path.as_ref().join("topic"), schema)?
        } else {
            Index::open_in_dir(path.as_ref().join("topic"))?
        };
        topic
            .tokenizers()
            .register("jieba", tantivy_jieba::JiebaTokenizer::new());
        let topic_fields = TopicFieldTable::try_from(&topic.schema())?;
        Ok(IndexService {
            topic,
            topic_fields,
        })
    }
    fn make_idx_topic(&self, topic: TopicDisplay) -> anyhow::Result<()> {
        let mut index_writer = self.topic.writer(50_000_000)?;
        let term = Term::from_field_i64(self.topic_fields.id, topic.id);
        index_writer.delete_term(term);
        info!("Building topic index: {}", topic.title);
        index_writer.add_document(doc!(
            self.topic_fields.id => topic.id,
            self.topic_fields.date => topic.created_at.to_rfc3339(),
            self.topic_fields.title => topic.title,
            self.topic_fields.content => topic.content_plain,// 使用纯文本构建索引
            self.topic_fields.username => topic.username,
            self.topic_fields.node => topic.node_name,
            self.topic_fields.node_slug => topic.node_slug,
        ))?;
        index_writer.commit()?;
        Ok(())
    }
    fn topic_search(&self, keyword: &str) -> anyhow::Result<Vec<TopicItem>> {
        let reader = self.topic.reader()?;
        let searcher = reader.searcher();
        let query_parser = QueryParser::for_index(
            &self.topic,
            vec![self.topic_fields.title, self.topic_fields.content],
        );
        let query = query_parser.parse_query(keyword)?;
        let top_docs: Vec<(Score, DocAddress)> =
            searcher.search(&query, &TopDocs::with_limit(100))?;
        let mut result = vec![];
        info!("Searching topics: {}, results: {}", keyword, top_docs.len());
        for (_score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc::<TantivyDocument>(doc_address)?;
            match TopicItem::try_from((&self.topic_fields, retrieved_doc)) {
                Ok(doc) => {
                    result.push(doc);
                }
                Err(err) => {
                    return Err(anyhow!(err));
                }
            }
        }
        Ok(result)
    }
    fn pure_idx_topic(&self, id: i64) -> anyhow::Result<()> {
        let term = Term::from_field_i64(self.topic_fields.id, id);
        let index_writer: IndexWriter = self.topic.writer(50_000_000)?;
        index_writer.delete_term(term);
        Ok(())
    }
}
