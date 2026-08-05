//! Data models matching the Hammer Editor protocol types.
//! Every serializable type here matches a Kotlin type in `base/src/commonMain`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

// ── HTTP Headers ──────────────────────────────────────────────

pub const HEADER_PROTOCOL_VERSION: &str = "X-Hammer-Protocol-Version";
pub const HEADER_SYNC_ID: &str = "X-Sync-Id";
pub const HEADER_ENTITY_HASH: &str = "X-Entity-Hash";
pub const HEADER_ORIGINAL_HASH: &str = "X-Original-Hash";
pub const HEADER_ENTITY_TYPE: &str = "X-Entity-Type";
pub const PROTOCOL_VERSION: u16 = 3;

// ── Base IDs ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ProjectId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct IdeaId(pub String);

// ── Account / Projects Sync ───────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiProjectDefinition {
    pub name: String,
    pub uuid: ProjectId,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeginProjectsSyncResponse {
    pub sync_id: String,
    pub projects: Vec<ApiProjectDefinition>,
    pub deleted_projects: Vec<ProjectId>,
    #[serde(default)]
    pub ideas_state_hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectResponse {
    pub project_id: ProjectId,
    pub already_existed: bool,
}

// ── Project Sync ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSynchronizationBegan {
    pub sync_id: String,
    pub last_sync: DateTime<Utc>,
    pub last_id: i32,
    pub id_sequence: Vec<i32>,
    pub deleted_ids: Vec<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteIdsResponse {
    pub deleted: bool,
}

// ── Client Entity State ───────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientEntityState {
    pub entities: Vec<EntityHash>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EntityHash {
    pub id: i32,
    pub hash: String,
}

// ── Entity Types ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityType {
    #[serde(rename = "scene")]
    Scene,
    #[serde(rename = "note")]
    Note,
    #[serde(rename = "timeline_event")]
    TimelineEvent,
    #[serde(rename = "encyclopedia_entry")]
    EncyclopediaEntry,
    #[serde(rename = "scene_draft")]
    SceneDraft,
}

impl EntityType {
    pub fn as_header_value(&self) -> &'static str {
        match self {
            EntityType::Scene => "scene",
            EntityType::Note => "note",
            EntityType::TimelineEvent => "timeline_event",
            EntityType::EncyclopediaEntry => "encyclopedia_entry",
            EntityType::SceneDraft => "scene_draft",
        }
    }
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_header_value())
    }
}

// ── ApiProjectEntity ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ApiProjectEntity {
    #[serde(rename = "scene")]
    Scene(SceneEntity),
    #[serde(rename = "note")]
    Note(NoteEntity),
    #[serde(rename = "timeline_event")]
    TimelineEvent(TimelineEventEntity),
    #[serde(rename = "encyclopedia_entry")]
    EncyclopediaEntry(EncyclopediaEntryEntity),
    #[serde(rename = "scene_draft")]
    SceneDraft(SceneDraftEntity),
}

impl ApiProjectEntity {
    pub fn entity_type(&self) -> EntityType {
        match self {
            ApiProjectEntity::Scene(_) => EntityType::Scene,
            ApiProjectEntity::Note(_) => EntityType::Note,
            ApiProjectEntity::TimelineEvent(_) => EntityType::TimelineEvent,
            ApiProjectEntity::EncyclopediaEntry(_) => EntityType::EncyclopediaEntry,
            ApiProjectEntity::SceneDraft(_) => EntityType::SceneDraft,
        }
    }

    pub fn id(&self) -> i32 {
        match self {
            ApiProjectEntity::Scene(e) => e.id,
            ApiProjectEntity::Note(e) => e.id,
            ApiProjectEntity::TimelineEvent(e) => e.id,
            ApiProjectEntity::EncyclopediaEntry(e) => e.id,
            ApiProjectEntity::SceneDraft(e) => e.id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneEntity {
    pub id: i32,
    #[serde(default)]
    pub scene_type: SceneType,
    #[serde(default)]
    pub order: i32,
    pub name: String,
    #[serde(default)]
    pub path: Vec<i32>,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub outline: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub confirmed_references: BTreeSet<i32>,
    #[serde(default)]
    pub dismissed_references: BTreeSet<i32>,
    #[serde(default)]
    pub tags: BTreeSet<String>,
    pub created: Option<DateTime<Utc>>,
    #[serde(rename = "lastEdited")]
    pub last_edited: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneType {
    #[serde(rename = "Scene")]
    Scene,
    #[serde(rename = "Group")]
    Group,
}

impl Default for SceneType {
    fn default() -> Self {
        SceneType::Scene
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteEntity {
    pub id: i32,
    pub content: String,
    pub created: DateTime<Utc>,
    #[serde(default)]
    pub tags: BTreeSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEventEntity {
    pub id: i32,
    #[serde(default)]
    pub order: i32,
    pub date: Option<String>,
    pub content: String,
    #[serde(default)]
    pub tags: BTreeSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncyclopediaEntryEntity {
    pub id: i32,
    pub name: String,
    #[serde(rename = "entryType")]
    pub entry_type: String,
    pub text: String,
    #[serde(default)]
    pub tags: BTreeSet<String>,
    pub image: Option<EncyclopediaImage>,
    #[serde(default)]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncyclopediaImage {
    pub base64: String,
    #[serde(rename = "fileExtension")]
    pub file_extension: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneDraftEntity {
    pub id: i32,
    pub scene_id: i32,
    pub created: DateTime<Utc>,
    pub name: String,
    pub content: String,
}

// ── Sync Probe ────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectHashItem {
    pub project_id: ProjectId,
    pub hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectsSyncProbeRequest {
    pub projects: Vec<ProjectHashItem>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectsSyncProbeResponse {
    pub unchanged_projects: Vec<ProjectId>,
}

// ── Project Data ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<ProjectTheme>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word_count_goal: Option<WordCountGoal>,
    #[serde(default)]
    pub tags: BTreeSet<String>,
}

impl Default for ProjectData {
    fn default() -> Self {
        ProjectData {
            author_name: None,
            theme: None,
            word_count_goal: None,
            tags: BTreeSet::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectTheme {
    pub primary: String,
    pub secondary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WordCountGoal {
    pub cadence: Cadence,
    pub count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Cadence {
    #[serde(rename = "DAY")]
    Day,
    #[serde(rename = "WEEK")]
    Week,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDataDto {
    pub data: ProjectData,
    pub hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDataUploadRequest {
    pub data: ProjectData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDataConflictDto {
    pub server: ProjectData,
    pub server_hash: String,
}

// ── Ideas ─────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdeaHashItem {
    pub id: IdeaId,
    pub hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdeasSyncStateResponse {
    pub ideas: Vec<IdeaHashItem>,
    pub deleted_ideas: Vec<IdeaId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryIdea {
    pub text: String,
    #[serde(default)]
    pub tags: BTreeSet<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedIdeaDto {
    pub idea: StoryIdea,
    pub hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdeaUploadRequest {
    pub idea: StoryIdea,
    pub hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdeaConflictDto {
    pub server: StoryIdea,
    pub server_hash: String,
}

// ── Writing Activity ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WritingSession {
    pub started_at: DateTime<Utc>,
    #[serde(default)]
    pub ended_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub words_written: i64,
    #[serde(default)]
    pub sealed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceLog {
    pub sessions: Vec<WritingSession>,
}

// ── Auth ──────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Token {
    #[serde(rename = "userId")]
    pub user_id: i64,
    pub auth: String,
    pub refresh: String,
}

#[derive(Debug, Serialize)]
pub struct LoginRequest<'a> {
    pub email: &'a str,
    pub password: &'a str,
    #[serde(rename = "installId")]
    pub install_id: &'a str,
}
