//! HTTP client for the Hammer Editor sync server.

use crate::models::*;
use anyhow::Result;
use reqwest::{header, Client as HttpClient, Response, StatusCode};
use std::io::Write;

/// High-level client wrapping the Hammer sync API.
pub struct HammerClient {
    http: HttpClient,
    server_url: String,
    auth_token: Option<String>,
}

impl HammerClient {
    pub fn new(server_url: String) -> Self {
        let http = HttpClient::builder()
            .gzip(true)
            .build()
            .expect("build reqwest client");

        Self {
            http,
            server_url: server_url.trim_end_matches('/').to_string(),
            auth_token: None,
        }
    }

    pub fn set_token(&mut self, token: &str) {
        self.auth_token = Some(format!("Bearer {}", token));
    }

    pub fn has_token(&self) -> bool {
        self.auth_token.is_some()
    }

    // ── helpers ───────────────────────────────────────────────

    fn protocol_header() -> (&'static str, String) {
        (HEADER_PROTOCOL_VERSION, PROTOCOL_VERSION.to_string())
    }

    fn auth_header(&self) -> Option<header::HeaderValue> {
        self.auth_token
            .as_ref()
            .map(|t| header::HeaderValue::from_str(t).expect("valid header value"))
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}/api{}", self.server_url, path)
    }

    async fn check_response(resp: Response) -> Result<Response> {
        let status = resp.status();
        if status.is_success()
            || matches!(
                status,
                StatusCode::CONFLICT
                    | StatusCode::NOT_FOUND
                    | StatusCode::GONE
                    | StatusCode::NOT_MODIFIED
                    | StatusCode::NO_CONTENT
            )
        {
            Ok(resp)
        } else {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("HTTP {}: {}", status.as_u16(), body)
        }
    }

    // ── Auth ──────────────────────────────────────────────────

    pub async fn login(&self, email: &str, password: &str, install_id: &str) -> Result<Token> {
        let body = LoginRequest {
            email,
            password,
            install_id,
        };
        let resp = self
            .http
            .post(format!("{}/api/accounts/login", self.server_url))
            .json(&body)
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(resp.json().await?)
        } else {
            let text = resp.text().await?;
            anyhow::bail!("Login failed: {}", text)
        }
    }

    // ── Account / Projects Sync ───────────────────────────────

    /// POST /api/projects/{userId}/begin_sync
    pub async fn begin_projects_sync(&self, user_id: i64) -> Result<BeginProjectsSyncResponse> {
        let url = self.api_url(&format!("/projects/{}/begin_sync", user_id));
        let resp = self
            .http
            .post(&url)
            .header(header::AUTHORIZATION, self.auth_header().unwrap())
            .header(HEADER_PROTOCOL_VERSION, PROTOCOL_VERSION.to_string())
            .send()
            .await?;
        Ok(Self::check_response(resp).await?.json().await?)
    }

    /// POST /api/projects/{userId}/end_sync
    pub async fn end_projects_sync(&self, user_id: i64, sync_id: &str) -> Result<()> {
        let url = self.api_url(&format!("/projects/{}/end_sync", user_id));
        let resp = self
            .http
            .post(&url)
            .header(header::AUTHORIZATION, self.auth_header().unwrap())
            .header(HEADER_PROTOCOL_VERSION, PROTOCOL_VERSION.to_string())
            .header(HEADER_SYNC_ID, sync_id)
            .send()
            .await?;
        Self::check_response(resp).await?;
        Ok(())
    }

    /// POST /api/projects/{userId}/create
    pub async fn create_project(
        &self,
        user_id: i64,
        sync_id: &str,
        project_name: &str,
    ) -> Result<CreateProjectResponse> {
        let url = self.api_url(&format!("/projects/{}/create", user_id));
        let resp = self
            .http
            .post(&url)
            .header(header::AUTHORIZATION, self.auth_header().unwrap())
            .header(HEADER_PROTOCOL_VERSION, PROTOCOL_VERSION.to_string())
            .header(HEADER_SYNC_ID, sync_id)
            .query(&[("projectName", project_name)])
            .send()
            .await?;
        Ok(Self::check_response(resp).await?.json().await?)
    }

    /// POST /api/projects/{userId}/delete
    pub async fn delete_project(
        &self,
        user_id: i64,
        sync_id: &str,
        project_id: &str,
    ) -> Result<()> {
        let url = self.api_url(&format!("/projects/{}/{}/delete", user_id, project_id));
        let resp = self
            .http
            .post(&url)
            .header(header::AUTHORIZATION, self.auth_header().unwrap())
            .header(HEADER_PROTOCOL_VERSION, PROTOCOL_VERSION.to_string())
            .header(HEADER_SYNC_ID, sync_id)
            .send()
            .await?;
        Self::check_response(resp).await?;
        Ok(())
    }

    /// POST /api/projects/{userId}/rename
    pub async fn rename_project(
        &self,
        user_id: i64,
        sync_id: &str,
        project_id: &str,
        new_name: &str,
    ) -> Result<()> {
        let url = self.api_url(&format!("/projects/{}/{}/rename", user_id, project_id));
        let resp = self
            .http
            .post(&url)
            .header(header::AUTHORIZATION, self.auth_header().unwrap())
            .header(HEADER_PROTOCOL_VERSION, PROTOCOL_VERSION.to_string())
            .header(HEADER_SYNC_ID, sync_id)
            .query(&[("projectName", new_name)])
            .send()
            .await?;
        Self::check_response(resp).await?;
        Ok(())
    }

    // ── Sync Probe ────────────────────────────────────────────

    /// POST /api/projects/{userId}/sync_probe
    pub async fn sync_probe(
        &self,
        user_id: i64,
        projects: Vec<ProjectHashItem>,
    ) -> Result<ProjectsSyncProbeResponse> {
        let url = self.api_url(&format!("/projects/{}/sync_probe", user_id));
        let body = ProjectsSyncProbeRequest { projects };
        let resp = self
            .http
            .post(&url)
            .header(header::AUTHORIZATION, self.auth_header().unwrap())
            .header(HEADER_PROTOCOL_VERSION, PROTOCOL_VERSION.to_string())
            .json(&body)
            .send()
            .await?;
        Ok(Self::check_response(resp).await?.json().await?)
    }

    // ── Project Sync ──────────────────────────────────────────

    /// POST /api/project/{userId}/{projectId}/begin_sync
    pub async fn begin_project_sync(
        &self,
        user_id: i64,
        project_id: &str,
        client_state: Option<&ClientEntityState>,
    ) -> Result<ProjectSynchronizationBegan> {
        let url = self.api_url(&format!("/project/{}/{}/begin_sync", user_id, project_id));

        let body = if let Some(state) = client_state {
            let json_str = serde_json::to_vec(state)?;
            let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
            gz.write_all(&json_str)?;
            gz.finish()?
        } else {
            Vec::new()
        };

        let resp = self
            .http
            .post(&url)
            .header(header::AUTHORIZATION, self.auth_header().unwrap())
            .header(HEADER_PROTOCOL_VERSION, PROTOCOL_VERSION.to_string())
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .body(body)
            .send()
            .await?;
        Ok(Self::check_response(resp).await?.json().await?)
    }

    /// POST /api/project/{userId}/{projectId}/end_sync
    pub async fn end_project_sync(
        &self,
        user_id: i64,
        project_id: &str,
        sync_id: &str,
        last_sync: Option<&str>,
        last_id: Option<i32>,
    ) -> Result<()> {
        let url = self.api_url(&format!("/project/{}/{}/end_sync", user_id, project_id));

        let mut form = Vec::new();
        if let Some(ls) = last_sync {
            form.push(("lastSync", ls.to_string()));
        }
        if let Some(li) = last_id {
            form.push(("lastId", li.to_string()));
        }

        let resp = self
            .http
            .post(&url)
            .header(header::AUTHORIZATION, self.auth_header().unwrap())
            .header(HEADER_PROTOCOL_VERSION, PROTOCOL_VERSION.to_string())
            .header(HEADER_SYNC_ID, sync_id)
            .form(&form)
            .send()
            .await?;
        Self::check_response(resp).await?;
        Ok(())
    }

    // ── Entities ──────────────────────────────────────────────

    /// POST /api/project/{userId}/{projectId}/upload_entity/{entityId}
    pub async fn upload_entity(
        &self,
        user_id: i64,
        project_id: &str,
        entity: &ApiProjectEntity,
        sync_id: &str,
        original_hash: Option<&str>,
        force: bool,
    ) -> Result<Option<ApiProjectEntity>> {
        let url = self.api_url(&format!(
            "/project/{}/{}/upload_entity/{}",
            user_id,
            project_id,
            entity.id()
        ));

        let mut req = self
            .http
            .post(&url)
            .header(header::AUTHORIZATION, self.auth_header().unwrap())
            .header(HEADER_PROTOCOL_VERSION, PROTOCOL_VERSION.to_string())
            .header(HEADER_SYNC_ID, sync_id)
            .header(HEADER_ENTITY_TYPE, entity.entity_type().as_header_value())
            .json(entity);

        if let Some(hash) = original_hash {
            req = req.header(HEADER_ORIGINAL_HASH, hash);
        }
        if force {
            req = req.query(&[("force", "true")]);
        }

        let resp = req.send().await?;
        let status = resp.status();

        if status == StatusCode::CONFLICT {
            Ok(Some(resp.json().await?))
        } else if status.is_success() {
            Ok(None)
        } else {
            let body = resp.text().await?;
            anyhow::bail!("Upload entity failed ({}): {}", status.as_u16(), body)
        }
    }

    /// GET /api/project/{userId}/{projectId}/download_entity/{entityId}
    pub async fn download_entity(
        &self,
        user_id: i64,
        project_id: &str,
        entity_id: i32,
        sync_id: &str,
        entity_hash: Option<&str>,
    ) -> Result<Option<ApiProjectEntity>> {
        let url = self.api_url(&format!(
            "/project/{}/{}/download_entity/{}",
            user_id, project_id, entity_id
        ));

        let mut req = self
            .http
            .get(&url)
            .header(header::AUTHORIZATION, self.auth_header().unwrap())
            .header(HEADER_PROTOCOL_VERSION, PROTOCOL_VERSION.to_string())
            .header(HEADER_SYNC_ID, sync_id);

        if let Some(hash) = entity_hash {
            req = req.header(HEADER_ENTITY_HASH, hash);
        }

        let resp = req.send().await?;
        let status = resp.status();

        if status == StatusCode::NOT_MODIFIED {
            return Ok(None); // already in sync
        }
        if status.is_success() {
            Ok(Some(resp.json().await?))
        } else {
            let body = resp.text().await?;
            anyhow::bail!("Download entity failed ({}): {}", status.as_u16(), body)
        }
    }

    /// GET /api/project/{userId}/{projectId}/delete_entity/{entityId}
    pub async fn delete_entity(
        &self,
        user_id: i64,
        project_id: &str,
        entity_id: i32,
        sync_id: &str,
    ) -> Result<bool> {
        let url = self.api_url(&format!(
            "/project/{}/{}/delete_entity/{}",
            user_id, project_id, entity_id
        ));

        let resp = self
            .http
            .get(&url)
            .header(header::AUTHORIZATION, self.auth_header().unwrap())
            .header(HEADER_PROTOCOL_VERSION, PROTOCOL_VERSION.to_string())
            .header(HEADER_SYNC_ID, sync_id)
            .send()
            .await?;

        let d: DeleteIdsResponse = Self::check_response(resp).await?.json().await?;
        Ok(d.deleted)
    }

    // ── Project Data ──────────────────────────────────────────

    /// GET /api/project/{userId}/{projectId}/project_data
    pub async fn get_project_data(
        &self,
        user_id: i64,
        project_id: &str,
    ) -> Result<Option<ProjectDataDto>> {
        let url = self.api_url(&format!("/project/{}/{}/project_data", user_id, project_id));

        let resp = self
            .http
            .get(&url)
            .header(header::AUTHORIZATION, self.auth_header().unwrap())
            .header(HEADER_PROTOCOL_VERSION, PROTOCOL_VERSION.to_string())
            .send()
            .await?;

        let status = resp.status();
        if status == StatusCode::NO_CONTENT {
            return Ok(None);
        }
        Ok(Some(Self::check_response(resp).await?.json().await?))
    }

    /// POST /api/project/{userId}/{projectId}/project_data
    pub async fn upload_project_data(
        &self,
        user_id: i64,
        project_id: &str,
        data: &ProjectData,
        original_hash: Option<&str>,
        hash: Option<&str>,
    ) -> Result<Option<ProjectDataConflictDto>> {
        let url = self.api_url(&format!("/project/{}/{}/project_data", user_id, project_id));

        let body = ProjectDataUploadRequest {
            data: data.clone(),
            original_hash: original_hash.map(String::from),
            hash: hash.map(String::from),
        };

        let resp = self
            .http
            .post(&url)
            .header(header::AUTHORIZATION, self.auth_header().unwrap())
            .header(HEADER_PROTOCOL_VERSION, PROTOCOL_VERSION.to_string())
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if status == StatusCode::CONFLICT {
            Ok(Some(resp.json().await?))
        } else if status.is_success() {
            Ok(None)
        } else {
            let text = resp.text().await?;
            anyhow::bail!("Upload project data failed ({}): {}", status.as_u16(), text)
        }
    }

    // ── Writing Activity ──────────────────────────────────────

    /// GET /api/project/{userId}/{projectId}/writing_activity
    pub async fn get_writing_activity(
        &self,
        user_id: i64,
        project_id: &str,
    ) -> Result<std::collections::HashMap<String, DeviceLog>> {
        let url = self.api_url(&format!(
            "/project/{}/{}/writing_activity",
            user_id, project_id
        ));

        let resp = self
            .http
            .get(&url)
            .header(header::AUTHORIZATION, self.auth_header().unwrap())
            .header(HEADER_PROTOCOL_VERSION, PROTOCOL_VERSION.to_string())
            .send()
            .await?;
        Ok(Self::check_response(resp).await?.json().await?)
    }

    // ── Ideas ─────────────────────────────────────────────────

    /// POST /api/ideas/{userId}/state
    pub async fn ideas_sync_state(
        &self,
        user_id: i64,
        sync_id: &str,
    ) -> Result<IdeasSyncStateResponse> {
        let url = self.api_url(&format!("/ideas/{}/state", user_id));
        let resp = self
            .http
            .post(&url)
            .header(header::AUTHORIZATION, self.auth_header().unwrap())
            .header(HEADER_PROTOCOL_VERSION, PROTOCOL_VERSION.to_string())
            .header(HEADER_SYNC_ID, sync_id)
            .send()
            .await?;
        Ok(Self::check_response(resp).await?.json().await?)
    }

    /// GET /api/ideas/{userId}/idea/{ideaId}
    pub async fn download_idea(
        &self,
        user_id: i64,
        idea_id: &str,
        sync_id: &str,
    ) -> Result<SavedIdeaDto> {
        let url = self.api_url(&format!("/ideas/{}/idea/{}", user_id, idea_id));
        let resp = self
            .http
            .get(&url)
            .header(header::AUTHORIZATION, self.auth_header().unwrap())
            .header(HEADER_PROTOCOL_VERSION, PROTOCOL_VERSION.to_string())
            .header(HEADER_SYNC_ID, sync_id)
            .send()
            .await?;
        Ok(Self::check_response(resp).await?.json().await?)
    }

    /// POST /api/ideas/{userId}/idea/{ideaId}
    pub async fn upload_idea(
        &self,
        user_id: i64,
        idea_id: &str,
        idea: &StoryIdea,
        hash: &str,
        original_hash: Option<&str>,
        sync_id: &str,
    ) -> Result<Option<IdeaConflictDto>> {
        let url = self.api_url(&format!("/ideas/{}/idea/{}", user_id, idea_id));
        let body = IdeaUploadRequest {
            idea: idea.clone(),
            hash: hash.to_string(),
            original_hash: original_hash.map(String::from),
        };

        let resp = self
            .http
            .post(&url)
            .header(header::AUTHORIZATION, self.auth_header().unwrap())
            .header(HEADER_PROTOCOL_VERSION, PROTOCOL_VERSION.to_string())
            .header(HEADER_SYNC_ID, sync_id)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if status == StatusCode::CONFLICT {
            Ok(Some(resp.json().await?))
        } else if status.is_success() {
            Ok(None)
        } else {
            let text = resp.text().await?;
            anyhow::bail!("Upload idea failed ({}): {}", status.as_u16(), text)
        }
    }

    /// POST /api/ideas/{userId}/idea/{ideaId}/delete
    pub async fn delete_idea(&self, user_id: i64, idea_id: &str, sync_id: &str) -> Result<()> {
        let url = self.api_url(&format!("/ideas/{}/idea/{}/delete", user_id, idea_id));
        let resp = self
            .http
            .post(&url)
            .header(header::AUTHORIZATION, self.auth_header().unwrap())
            .header(HEADER_PROTOCOL_VERSION, PROTOCOL_VERSION.to_string())
            .header(HEADER_SYNC_ID, sync_id)
            .send()
            .await?;
        Self::check_response(resp).await?;
        Ok(())
    }
}
