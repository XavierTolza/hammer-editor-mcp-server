//! Tool implementations for the MCP server.

use super::server::McpServer;
use crate::models::*;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use uuid::Uuid;

impl McpServer {
    pub(crate) async fn tool_login(&mut self, args: &Value) -> Result<Value, String> {
        let email = args["email"].as_str().ok_or("email required")?;
        let password = args["password"].as_str().ok_or("password required")?;
        let install_id = Uuid::new_v4().to_string();
        let token = self
            .client
            .login(email, password, &install_id)
            .await
            .map_err(|e| format!("Login failed: {}", e))?;
        self.user_id = Some(token.user_id);
        self.client.set_token(&token.auth);
        Ok(
            json!({"success":true,"user_id":token.user_id,"auth_token":token.auth,"refresh_token":token.refresh}),
        )
    }

    pub(crate) fn tool_set_user_id(&mut self, args: &Value) -> Result<Value, String> {
        let uid = args["user_id"]
            .as_i64()
            .ok_or("user_id (integer) required")?;
        self.user_id = Some(uid);
        Ok(json!({"success":true,"user_id":uid}))
    }

    pub(crate) async fn tool_begin_account_sync(&mut self) -> Result<Value, String> {
        let uid = self.require_user()?;

        // End any stale server-side session before starting a new one
        if let Some(sid) = self.account_sync_id.take() {
            let _ = self.client.end_projects_sync(uid, &sid).await;
        }

        // End any active project syncs first (server doesn't allow concurrent sessions)
        let pids: Vec<String> = self.project_sessions.keys().cloned().collect();
        for pid in &pids {
            if let Some(session) = self.project_sessions.remove(pid) {
                let _ = self
                    .client
                    .end_project_sync(
                        uid,
                        pid,
                        &session.sync_id,
                        session.last_sync.as_deref(),
                        Some(session.last_id),
                    )
                    .await;
            }
        }

        let resp = self
            .client
            .begin_projects_sync(uid)
            .await
            .map_err(|e| e.to_string())?;
        self.account_sync_id = Some(resp.sync_id.clone());
        serde_json::to_value(&resp).map_err(|e| e.to_string())
    }

    pub(crate) async fn tool_end_account_sync(&mut self) -> Result<Value, String> {
        let uid = self.require_user()?;
        let sid = self
            .account_sync_id
            .take()
            .ok_or("No active account sync session")?;
        self.client
            .end_projects_sync(uid, &sid)
            .await
            .map_err(|e| e.to_string())?;
        Ok(json!({"success":true}))
    }

    pub(crate) async fn tool_create_project(&mut self, args: &Value) -> Result<Value, String> {
        let uid = self.require_user()?;
        let sid = self.require_account_sync()?;
        let name = args["project_name"]
            .as_str()
            .ok_or("project_name required")?;
        let resp = self
            .client
            .create_project(uid, sid, name)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_value(&resp).map_err(|e| e.to_string())
    }

    pub(crate) async fn tool_delete_project(&mut self, args: &Value) -> Result<Value, String> {
        let uid = self.require_user()?;
        let sid = self.require_account_sync()?;
        let pid = args["project_id"].as_str().ok_or("project_id required")?;
        self.client
            .delete_project(uid, sid, pid)
            .await
            .map_err(|e| e.to_string())?;
        self.project_sessions.remove(pid);
        Ok(json!({"success":true}))
    }

    pub(crate) async fn tool_rename_project(&mut self, args: &Value) -> Result<Value, String> {
        let uid = self.require_user()?;
        let sid = self.require_account_sync()?;
        let pid = args["project_id"].as_str().ok_or("project_id required")?;
        let name = args["new_name"].as_str().ok_or("new_name required")?;
        self.client
            .rename_project(uid, sid, pid, name)
            .await
            .map_err(|e| e.to_string())?;
        Ok(json!({"success":true}))
    }

    pub(crate) async fn tool_begin_project_sync(&mut self, args: &Value) -> Result<Value, String> {
        let uid = self.require_user()?;
        let pid = args["project_id"].as_str().ok_or("project_id required")?;

        // End account sync first (server doesn't allow concurrent sessions)
        if let Some(sid) = self.account_sync_id.take() {
            let _ = self.client.end_projects_sync(uid, &sid).await;
        }

        let client_state = if let Some(hashes) = args["entity_hashes"].as_array() {
            let entities: Vec<EntityHash> = hashes
                .iter()
                .filter_map(|h| {
                    Some(EntityHash {
                        id: h["id"].as_i64()? as i32,
                        hash: h["hash"].as_str()?.to_string(),
                    })
                })
                .collect();
            if entities.is_empty() {
                None
            } else {
                Some(ClientEntityState { entities })
            }
        } else {
            None
        };
        let resp = self
            .client
            .begin_project_sync(uid, pid, client_state.as_ref())
            .await
            .map_err(|e| e.to_string())?;
        self.project_sessions.insert(
            pid.to_string(),
            super::server::ProjectSyncSession {
                sync_id: resp.sync_id.clone(),
                last_id: resp.last_id,
                last_sync: Some(resp.last_sync.to_rfc3339()),
                deleted_ids: resp.deleted_ids.clone(),
            },
        );
        serde_json::to_value(&resp).map_err(|e| e.to_string())
    }

    pub(crate) async fn tool_end_project_sync(&mut self, args: &Value) -> Result<Value, String> {
        let uid = self.require_user()?;
        let pid = args["project_id"].as_str().ok_or("project_id required")?;
        let session = self.project_sessions.remove(pid).ok_or("No active project sync session for this project. Call hammer_begin_project_sync first.")?;
        let last_sync = args["last_sync"].as_str();
        let last_id = args["last_id"].as_i64().map(|v| v as i32);
        self.client
            .end_project_sync(uid, pid, &session.sync_id, last_sync, last_id)
            .await
            .map_err(|e| e.to_string())?;
        Ok(json!({"success":true}))
    }

    pub(crate) async fn tool_download_entity(&self, args: &Value) -> Result<Value, String> {
        let uid = self.require_user()?;
        let pid = args["project_id"].as_str().ok_or("project_id required")?;
        let eid = args["entity_id"].as_i64().ok_or("entity_id required")? as i32;
        let hash = args["entity_hash"].as_str();
        let session = self.require_project_sync(pid)?;
        let entity = self
            .client
            .download_entity(uid, pid, eid, &session.sync_id, hash)
            .await
            .map_err(|e| e.to_string())?;
        match entity {
            Some(e) => serde_json::to_value(&e).map_err(|e| e.to_string()),
            None => {
                Ok(json!({"status":"not_modified","message":"Entity unchanged since last sync"}))
            }
        }
    }

    pub(crate) async fn tool_upload_entity(&self, args: &Value) -> Result<Value, String> {
        let uid = self.require_user()?;
        let pid = args["project_id"].as_str().ok_or("project_id required")?;
        let session = self.require_project_sync(pid)?;
        let force = args["force"].as_bool().unwrap_or(false);
        let original_hash = args["original_hash"].as_str();
        let entity: ApiProjectEntity = serde_json::from_value(args["entity"].clone())
            .map_err(|e| format!("Invalid entity JSON: {}", e))?;
        let conflict = self
            .client
            .upload_entity(uid, pid, &entity, &session.sync_id, original_hash, force)
            .await
            .map_err(|e| e.to_string())?;
        match conflict {
            Some(server_entity) => Ok(
                json!({"status":"conflict","message":"Entity was modified on the server since your last sync.","server_entity":server_entity}),
            ),
            None => Ok(json!({"status":"saved","success":true})),
        }
    }

    pub(crate) async fn tool_delete_entity(&self, args: &Value) -> Result<Value, String> {
        let uid = self.require_user()?;
        let pid = args["project_id"].as_str().ok_or("project_id required")?;
        let eid = args["entity_id"].as_i64().ok_or("entity_id required")? as i32;
        let session = self.require_project_sync(pid)?;
        let deleted = self
            .client
            .delete_entity(uid, pid, eid, &session.sync_id)
            .await
            .map_err(|e| e.to_string())?;
        Ok(json!({"success":true,"deleted":deleted}))
    }

    pub(crate) async fn tool_get_project_data(&self, args: &Value) -> Result<Value, String> {
        let uid = self.require_user()?;
        let pid = args["project_id"].as_str().ok_or("project_id required")?;
        let data = self
            .client
            .get_project_data(uid, pid)
            .await
            .map_err(|e| e.to_string())?;
        match data {
            Some(d) => serde_json::to_value(&d).map_err(|e| e.to_string()),
            None => Ok(json!({"status":"empty","data":null,"hash":null})),
        }
    }

    pub(crate) async fn tool_upload_project_data(&self, args: &Value) -> Result<Value, String> {
        let uid = self.require_user()?;
        let pid = args["project_id"].as_str().ok_or("project_id required")?;
        let ohash = args["original_hash"].as_str();
        let hash = args["hash"].as_str();
        let data: ProjectData = serde_json::from_value(args["data"].clone())
            .map_err(|e| format!("Invalid project data: {}", e))?;
        let conflict = self
            .client
            .upload_project_data(uid, pid, &data, ohash, hash)
            .await
            .map_err(|e| e.to_string())?;
        match conflict {
            Some(c) => {
                Ok(json!({"status":"conflict","server_data":c.server,"server_hash":c.server_hash}))
            }
            None => Ok(json!({"status":"saved","success":true})),
        }
    }

    pub(crate) async fn tool_get_ideas_state(&self) -> Result<Value, String> {
        let uid = self.require_user()?;
        let sid = self.require_account_sync()?;
        let state = self
            .client
            .ideas_sync_state(uid, sid)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_value(&state).map_err(|e| e.to_string())
    }

    pub(crate) async fn tool_download_idea(&self, args: &Value) -> Result<Value, String> {
        let uid = self.require_user()?;
        let sid = self.require_account_sync()?;
        let iid = args["idea_id"].as_str().ok_or("idea_id required")?;
        let idea = self
            .client
            .download_idea(uid, iid, sid)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_value(&idea).map_err(|e| e.to_string())
    }

    pub(crate) async fn tool_upload_idea(&self, args: &Value) -> Result<Value, String> {
        let uid = self.require_user()?;
        let sid = self.require_account_sync()?;
        let iid = args["idea_id"].as_str().ok_or("idea_id required")?;
        let text = args["text"].as_str().ok_or("text required")?;
        let tags: BTreeSet<String> = args["tags"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let hash = args["hash"].as_str().unwrap_or("");
        let ohash = args["original_hash"].as_str();
        let idea = StoryIdea {
            text: text.to_string(),
            tags,
        };
        let conflict = self
            .client
            .upload_idea(uid, iid, &idea, hash, ohash, sid)
            .await
            .map_err(|e| e.to_string())?;
        match conflict {
            Some(c) => {
                Ok(json!({"status":"conflict","server_idea":c.server,"server_hash":c.server_hash}))
            }
            None => Ok(json!({"status":"saved","success":true})),
        }
    }

    pub(crate) async fn tool_delete_idea(&self, args: &Value) -> Result<Value, String> {
        let uid = self.require_user()?;
        let sid = self.require_account_sync()?;
        let iid = args["idea_id"].as_str().ok_or("idea_id required")?;
        self.client
            .delete_idea(uid, iid, sid)
            .await
            .map_err(|e| e.to_string())?;
        Ok(json!({"success":true}))
    }

    pub(crate) async fn tool_get_writing_activity(&self, args: &Value) -> Result<Value, String> {
        let uid = self.require_user()?;
        let pid = args["project_id"].as_str().ok_or("project_id required")?;
        let activity = self
            .client
            .get_writing_activity(uid, pid)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_value(&activity).map_err(|e| e.to_string())
    }

    pub(crate) async fn tool_sync_probe(&self, args: &Value) -> Result<Value, String> {
        let uid = self.require_user()?;
        let projects: Vec<ProjectHashItem> = args["projects"]
            .as_array()
            .ok_or("projects array required")?
            .iter()
            .map(|p| ProjectHashItem {
                project_id: ProjectId(p["project_id"].as_str().unwrap_or("").to_string()),
                hash: p["hash"].as_str().unwrap_or("").to_string(),
            })
            .collect();
        let resp = self
            .client
            .sync_probe(uid, projects)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_value(&resp).map_err(|e| e.to_string())
    }

    pub(crate) fn tool_entity_schema(&self, args: &Value) -> Result<Value, String> {
        let typ = args["entity_type"].as_str().ok_or("entity_type required")?;
        let schema = match typ {
            "scene" => json!({"name":"Scene","entity_type":"scene","fields":{"id":"integer","scene_type":"\"Scene\"|\"Group\"","order":"integer","name":"string","path":"[integer]","content":"string (markdown)","outline":"string","notes":"string","archived":"boolean","confirmed_references":"[integer]","dismissed_references":"[integer]","tags":"[string]","created":"ISO 8601 datetime","lastEdited":"ISO 8601 datetime"}}),
            "note" => json!({"name":"Note","entity_type":"note","fields":{"id":"integer","content":"string (markdown)","created":"ISO 8601 datetime","tags":"[string]"}}),
            "timeline_event" => json!({"name":"TimelineEvent","entity_type":"timeline_event","fields":{"id":"integer","order":"integer","date":"string (free-form, nullable)","content":"string (markdown)","tags":"[string]"}}),
            "encyclopedia_entry" => json!({"name":"EncyclopediaEntry","entity_type":"encyclopedia_entry","fields":{"id":"integer","name":"string","entryType":"string (category)","text":"string (markdown)","tags":"[string]","image":"{base64,fileExtension} (nullable)","aliases":"[string]"}}),
            "scene_draft" => json!({"name":"SceneDraft","entity_type":"scene_draft","fields":{"id":"integer","sceneId":"integer","created":"ISO 8601 datetime","name":"string","content":"string (markdown)"}}),
            _ => return Err(format!("Unknown entity type: {}. Use: scene, note, timeline_event, encyclopedia_entry, scene_draft", typ)),
        };
        Ok(schema)
    }
}
