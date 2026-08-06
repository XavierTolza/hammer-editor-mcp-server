//! MCP server types, tool definitions, and tool implementations.

use crate::client::HammerClient;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

// ── JSON-RPC types ───────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

// ── Session state ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ProjectSyncSession {
    pub sync_id: String,
    pub last_id: i32,
    pub last_sync: Option<String>,
    pub deleted_ids: Vec<i32>,
}

/// MCP server state: the sync session manager and the Hammer HTTP client.
pub struct McpServer {
    pub client: HammerClient,
    pub(crate) user_id: Option<i64>,
    pub(crate) account_sync_id: Option<String>,
    pub(crate) project_sessions: HashMap<String, ProjectSyncSession>,
}

impl McpServer {
    pub fn new(server_url: String) -> Self {
        Self {
            client: HammerClient::new(server_url),
            user_id: None,
            account_sync_id: None,
            project_sessions: HashMap::new(),
        }
    }

    /// Apply connection settings from CLI arguments.
    ///
    /// Priority: an explicit `auth_token` is used directly; otherwise
    /// `email`/`password` are used to log in and obtain a token. A
    /// `user_id` may be provided to skip the look-up that login returns.
    pub async fn configure(&mut self, cfg: &crate::config::CliArgs) -> anyhow::Result<()> {
        if let Some(token) = &cfg.auth_token {
            self.client.set_token(token);
        } else if let (Some(email), Some(password)) = (&cfg.email, &cfg.password) {
            let install_id = cfg
                .install_id
                .clone()
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            let token = self.client.login(email, password, &install_id).await?;
            self.user_id = Some(token.user_id);
            self.client.set_token(&token.auth);
            tracing::info!("Authenticated as user {}", token.user_id);
        }

        if let Some(uid) = cfg.user_id {
            self.user_id = Some(uid);
        }

        if cfg.auth_token.is_none() && cfg.email.is_none() && cfg.user_id.is_none() {
            tracing::warn!(
                "No credentials provided. Use --email/--password, --auth-token or --user-id."
            );
        }
        Ok(())
    }

    /// Whether the server has a user id available for API calls.
    pub fn is_authenticated(&self) -> bool {
        self.user_id.is_some()
    }

    pub fn tool_definitions() -> Vec<Value> {
        vec![
            json!({"name":"hammer_begin_account_sync","description":"Begin account-level sync. Returns syncId, projects list, deleted projects, ideas state hash.","inputSchema":{"type":"object","properties":{},"required":[]}}),
            json!({"name":"hammer_end_account_sync","description":"End the current account-level sync session.","inputSchema":{"type":"object","properties":{},"required":[]}}),
            json!({"name":"hammer_create_project","description":"Create a new project. Requires active account sync.","inputSchema":{"type":"object","properties":{"project_name":{"type":"string"}},"required":["project_name"]}}),
            json!({"name":"hammer_delete_project","description":"Delete a project by UUID. Requires active account sync.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"}},"required":["project_id"]}}),
            json!({"name":"hammer_rename_project","description":"Rename a project. Requires active account sync.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"new_name":{"type":"string"}},"required":["project_id","new_name"]}}),
            json!({"name":"hammer_begin_project_sync","description":"Begin project sync. Returns syncId, lastId, lastSync, update sequence, deleted IDs. Optional entity_hashes for incremental sync.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"entity_hashes":{"type":"array","items":{"type":"object","properties":{"id":{"type":"integer"},"hash":{"type":"string"}},"required":["id","hash"]}}},"required":["project_id"]}}),
            json!({"name":"hammer_end_project_sync","description":"End project sync. Provide last_sync (ISO 8601) and last_id.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"last_sync":{"type":"string"},"last_id":{"type":"integer"}},"required":["project_id"]}}),
            json!({"name":"hammer_download_entity","description":"Download an entity by ID. Requires active project sync.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"entity_id":{"type":"integer"},"entity_hash":{"type":"string"}},"required":["project_id","entity_id"]}}),
            json!({"name":"hammer_upload_entity","description":"Upload/update an entity. Types: scene, note, timeline_event, encyclopedia_entry, scene_draft. Provide entity matching the type.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"entity_type":{"type":"string","enum":["scene","note","timeline_event","encyclopedia_entry","scene_draft"]},"entity":{"type":"object"},"original_hash":{"type":"string"},"force":{"type":"boolean"}},"required":["project_id","entity_type","entity"]}}),
            json!({"name":"hammer_delete_entity","description":"Delete an entity. Requires active project sync.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"entity_id":{"type":"integer"}},"required":["project_id","entity_id"]}}),
            json!({"name":"hammer_get_project_data","description":"Get project metadata (author, theme, word goal, tags).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"}},"required":["project_id"]}}),
            json!({"name":"hammer_upload_project_data","description":"Update project metadata.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"data":{"type":"object"},"original_hash":{"type":"string"},"hash":{"type":"string"}},"required":["project_id","data"]}}),
            json!({"name":"hammer_get_ideas_state","description":"Get story ideas sync state. Requires active account sync.","inputSchema":{"type":"object","properties":{},"required":[]}}),
            json!({"name":"hammer_download_idea","description":"Download a story idea by UUID. Requires active account sync.","inputSchema":{"type":"object","properties":{"idea_id":{"type":"string"}},"required":["idea_id"]}}),
            json!({"name":"hammer_upload_idea","description":"Upload/update a story idea. idea_id is a client-generated UUID.","inputSchema":{"type":"object","properties":{"idea_id":{"type":"string"},"text":{"type":"string"},"tags":{"type":"array","items":{"type":"string"}},"hash":{"type":"string"},"original_hash":{"type":"string"}},"required":["idea_id","text"]}}),
            json!({"name":"hammer_delete_idea","description":"Delete a story idea. Requires active account sync.","inputSchema":{"type":"object","properties":{"idea_id":{"type":"string"}},"required":["idea_id"]}}),
            json!({"name":"hammer_get_writing_activity","description":"Get writing stats for a project (all devices).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"}},"required":["project_id"]}}),
            json!({"name":"hammer_sync_probe","description":"Check which projects have changed (fast pre-check).","inputSchema":{"type":"object","properties":{"projects":{"type":"array","items":{"type":"object","properties":{"project_id":{"type":"string"},"hash":{"type":"string"}},"required":["project_id","hash"]}}},"required":["projects"]}}),
            json!({"name":"hammer_entity_schema","description":"Get JSON schema for an entity type.","inputSchema":{"type":"object","properties":{"entity_type":{"type":"string","enum":["scene","note","timeline_event","encyclopedia_entry","scene_draft"]}},"required":["entity_type"]}}),
        ]
    }

    pub async fn handle_call(&mut self, name: &str, args: &Value) -> Result<Value, String> {
        match name {
            "hammer_begin_account_sync" => self.tool_begin_account_sync().await,
            "hammer_end_account_sync" => self.tool_end_account_sync().await,
            "hammer_create_project" => self.tool_create_project(args).await,
            "hammer_delete_project" => self.tool_delete_project(args).await,
            "hammer_rename_project" => self.tool_rename_project(args).await,
            "hammer_begin_project_sync" => self.tool_begin_project_sync(args).await,
            "hammer_end_project_sync" => self.tool_end_project_sync(args).await,
            "hammer_download_entity" => self.tool_download_entity(args).await,
            "hammer_upload_entity" => self.tool_upload_entity(args).await,
            "hammer_delete_entity" => self.tool_delete_entity(args).await,
            "hammer_get_project_data" => self.tool_get_project_data(args).await,
            "hammer_upload_project_data" => self.tool_upload_project_data(args).await,
            "hammer_get_ideas_state" => self.tool_get_ideas_state().await,
            "hammer_download_idea" => self.tool_download_idea(args).await,
            "hammer_upload_idea" => self.tool_upload_idea(args).await,
            "hammer_delete_idea" => self.tool_delete_idea(args).await,
            "hammer_get_writing_activity" => self.tool_get_writing_activity(args).await,
            "hammer_sync_probe" => self.tool_sync_probe(args).await,
            "hammer_entity_schema" => self.tool_entity_schema(args),
            _ => Err(format!("Unknown tool: {}", name)),
        }
    }

    pub(crate) fn require_user(&self) -> Result<i64, String> {
        self.user_id
            .ok_or_else(|| "Not authenticated. Provide --email/--password, --auth-token, or --user-id on startup.".into())
    }
    pub(crate) fn require_account_sync(&self) -> Result<&str, String> {
        self.account_sync_id
            .as_deref()
            .ok_or_else(|| "No active account sync. Call hammer_begin_account_sync first.".into())
    }
    pub(crate) fn require_project_sync(&self, pid: &str) -> Result<&ProjectSyncSession, String> {
        self.project_sessions.get(pid).ok_or_else(|| {
            format!(
                "No active project sync for '{}'. Call hammer_begin_project_sync first.",
                pid
            )
        })
    }
}
