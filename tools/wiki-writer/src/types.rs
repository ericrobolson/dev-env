use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Claude,
    Codex,
    Fake,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    Small,
    Mid,
    Large,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Ingest,
    Survey,
    Extract,
    Audit,
    Resolve,
    Organize,
    Write,
    Link,
    Review,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemStatus {
    Pending,
    Done,
    Blocked,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PageKind {
    Leaf,
    Hub,
    Index,
    Overview,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageRef {
    pub source: String,
    pub page: u32,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub id: String,
    pub name: String,
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Page {
    pub id: u32,
    pub reference: PageRef,
    pub text: String,
    pub scanned: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TocEntry {
    pub title: String,
    pub source: String,
    pub page: u32,
    pub level: u8,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chunk {
    pub id: String,
    pub source: String,
    pub first_page: u32,
    pub last_page: u32,
    pub text: String,
    pub table_like: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntityType {
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Schema {
    pub entity_types: BTreeMap<String, EntityType>,
    pub namespaces: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactDraft {
    pub entity: String,
    pub entity_type: String,
    pub aliases: Vec<String>,
    pub fields: BTreeMap<String, String>,
    pub text: String,
    pub quote: String,
    pub pages: Vec<PageRef>,
    pub table_like: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fact {
    pub id: String,
    pub chunk: String,
    pub draft: FactDraft,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Notes {
    pub facts: Vec<Fact>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entity {
    pub id: String,
    pub name: String,
    pub entity_type: String,
    pub aliases: Vec<String>,
    pub fact_ids: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageSpec {
    pub id: String,
    pub title: String,
    pub kind: PageKind,
    pub entity_ids: Vec<String>,
    pub fact_ids: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Outline {
    pub pages: Vec<PageSpec>,
    pub gaps: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemRecord {
    pub input_hash: String,
    pub output_hash: Option<String>,
    pub output_path: Option<String>,
    pub outputs: BTreeMap<String, String>,
    pub status: ItemStatus,
    pub errors: Vec<String>,
    pub escalated: bool,
}
