//! Hammer Editor MCP Server — library crate.
//!
//! Re-exports the core modules for use by both the binary and integration tests.

pub mod client;
pub mod config;
pub mod models;
pub mod server;
pub mod tools;

pub mod mcp;

pub use config::CliArgs;

pub fn init_logging() {
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_target(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod integration_tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Test: MCP `tools/list` returns all expected tool definitions (no auth tools).
    #[test]
    fn tools_list_returns_all_tools() {
        let tools = server::McpServer::tool_definitions();
        let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(!tool_names.contains(&"hammer_login"));
        assert!(!tool_names.contains(&"hammer_set_user_id"));
        assert!(tool_names.contains(&"hammer_begin_account_sync"));
        assert!(tool_names.contains(&"hammer_begin_project_sync"));
        assert!(tool_names.contains(&"hammer_download_entity"));
        assert!(tool_names.contains(&"hammer_upload_entity"));
        assert!(tool_names.contains(&"hammer_get_project_data"));
        assert!(tool_names.contains(&"hammer_get_ideas_state"));
        assert!(tool_names.contains(&"hammer_get_writing_activity"));
        assert!(tool_names.contains(&"hammer_entity_schema"));
        assert_eq!(tools.len(), 19);
    }

    /// Test: entity schemas work.
    #[test]
    fn entity_schemas_are_valid() {
        let srv = server::McpServer::new("http://localhost".into());
        for etype in &[
            "scene",
            "note",
            "timeline_event",
            "encyclopedia_entry",
            "scene_draft",
        ] {
            assert!(srv
                .tool_entity_schema(&json!({"entity_type": etype}))
                .is_ok());
        }
        assert!(srv
            .tool_entity_schema(&json!({"entity_type": "bad"}))
            .is_err());
    }

    /// Test: configure rejects bad credentials.
    #[tokio::test]
    async fn configure_rejects_bad_credentials() {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/accounts/login"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({"error":"Invalid"})))
            .mount(&mock)
            .await;

        let mut srv = server::McpServer::new(mock.uri());
        let args = crate::CliArgs {
            server_url: mock.uri(),
            email: Some("b@t.com".into()),
            password: Some("w".into()),
            install_id: None,
            user_id: None,
            auth_token: None,
        };
        let r = srv.configure(&args).await;
        assert!(r.is_err());
    }

    /// Test: configure succeeds with valid credentials.
    #[tokio::test]
    async fn configure_succeeds_with_credentials() {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/accounts/login"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"userId":42,"auth":"tok","refresh":"ref"})),
            )
            .mount(&mock)
            .await;

        let mut srv = server::McpServer::new(mock.uri());
        let args = crate::CliArgs {
            server_url: mock.uri(),
            email: Some("u@t.com".into()),
            password: Some("p".into()),
            install_id: None,
            user_id: None,
            auth_token: None,
        };
        srv.configure(&args).await.unwrap();
        assert_eq!(srv.user_id, Some(42));
        assert!(srv.client.has_token());
    }

    /// Test: configure with auth token directly.
    #[tokio::test]
    async fn configure_with_auth_token() {
        let mut srv = server::McpServer::new("http://localhost".into());
        let args = crate::CliArgs {
            server_url: "http://localhost".into(),
            email: None,
            password: None,
            install_id: None,
            user_id: Some(99),
            auth_token: Some("mytoken".into()),
        };
        srv.configure(&args).await.unwrap();
        assert_eq!(srv.user_id, Some(99));
        assert!(srv.client.has_token());
    }

    /// Test: configure with user_id only.
    #[tokio::test]
    async fn configure_with_user_id_only() {
        let mut srv = server::McpServer::new("http://localhost".into());
        let args = crate::CliArgs {
            server_url: "http://localhost".into(),
            email: None,
            password: None,
            install_id: None,
            user_id: Some(77),
            auth_token: None,
        };
        srv.configure(&args).await.unwrap();
        assert_eq!(srv.user_id, Some(77));
    }

    /// Test: begin_account_sync works (pre-configured client).
    #[tokio::test]
    async fn begin_account_sync_works() {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/accounts/login"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"userId":1,"auth":"tok","refresh":"ref"})),
            )
            .mount(&mock)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/projects/1/begin_sync"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                json!({"syncId":"s1","projects":[{"name":"N","uuid":"u"}],"deletedProjects":[]}),
            ))
            .mount(&mock)
            .await;

        let mut srv = server::McpServer::new(mock.uri());
        let args = crate::CliArgs {
            server_url: mock.uri(),
            email: Some("u@t.com".into()),
            password: Some("p".into()),
            install_id: None,
            user_id: None,
            auth_token: None,
        };
        srv.configure(&args).await.unwrap();
        let r = srv.tool_begin_account_sync().await.unwrap();
        assert!(r.to_string().contains("s1"));
        assert!(r.to_string().contains("N"));
    }

    /// Test: create_project.
    #[tokio::test]
    async fn create_project_works() {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/accounts/login"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"userId":1,"auth":"tok","refresh":"ref"})),
            )
            .mount(&mock)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/projects/1/begin_sync"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"syncId":"s","projects":[],"deletedProjects":[]})),
            )
            .mount(&mock)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/projects/1/create"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"projectId":"p1","alreadyExisted":false})),
            )
            .mount(&mock)
            .await;

        let mut srv = server::McpServer::new(mock.uri());
        let args = crate::CliArgs {
            server_url: mock.uri(),
            email: Some("u@t.com".into()),
            password: Some("p".into()),
            install_id: None,
            user_id: None,
            auth_token: None,
        };
        srv.configure(&args).await.unwrap();
        srv.tool_begin_account_sync().await.unwrap();
        let r = srv
            .tool_create_project(&json!({"project_name":"NS"}))
            .await
            .unwrap();
        assert!(r.to_string().contains("p1"));
    }

    /// Test: begin_project_sync.
    #[tokio::test]
    async fn begin_project_sync_works() {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/accounts/login"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"userId":1,"auth":"tok","refresh":"ref"})),
            )
            .mount(&mock)
            .await;
        Mock::given(method("POST")).and(path("/api/project/1/p1/begin_sync"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"syncId":"ps","lastSync":"2024-01-01T00:00:00Z","lastId":5,"idSequence":[1,2,3],"deletedIds":[]})))
            .mount(&mock).await;

        let mut srv = server::McpServer::new(mock.uri());
        let args = crate::CliArgs {
            server_url: mock.uri(),
            email: Some("u@t.com".into()),
            password: Some("p".into()),
            install_id: None,
            user_id: None,
            auth_token: None,
        };
        srv.configure(&args).await.unwrap();
        let r = srv
            .tool_begin_project_sync(&json!({"project_id":"p1"}))
            .await
            .unwrap();
        assert!(r.to_string().contains("ps"));
        assert!(r.to_string().contains("\"idSequence\""));
    }

    /// Test: download_entity 304.
    #[tokio::test]
    async fn download_entity_not_modified() {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/accounts/login"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"userId":1,"auth":"tok","refresh":"ref"})),
            )
            .mount(&mock)
            .await;
        Mock::given(method("POST")).and(path("/api/project/1/p1/begin_sync"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"syncId":"ps","lastSync":"2024-01-01T00:00:00Z","lastId":10,"idSequence":[],"deletedIds":[]})))
            .mount(&mock).await;
        Mock::given(method("GET"))
            .and(path("/api/project/1/p1/download_entity/1"))
            .respond_with(ResponseTemplate::new(304))
            .mount(&mock)
            .await;

        let mut srv = server::McpServer::new(mock.uri());
        let args = crate::CliArgs {
            server_url: mock.uri(),
            email: Some("u@t.com".into()),
            password: Some("p".into()),
            install_id: None,
            user_id: None,
            auth_token: None,
        };
        srv.configure(&args).await.unwrap();
        srv.tool_begin_project_sync(&json!({"project_id":"p1"}))
            .await
            .unwrap();
        let r = srv
            .tool_download_entity(&json!({"project_id":"p1","entity_id":1,"entity_hash":"same"}))
            .await
            .unwrap();
        assert!(r.to_string().contains("not_modified"));
    }

    /// Test: end_project_sync works.
    #[tokio::test]
    async fn end_project_sync_works() {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/accounts/login"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"userId":1,"auth":"tok","refresh":"ref"})),
            )
            .mount(&mock)
            .await;
        Mock::given(method("POST")).and(path("/api/project/1/p1/begin_sync"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"syncId":"ps","lastSync":"2024-01-01T00:00:00Z","lastId":10,"idSequence":[],"deletedIds":[]})))
            .mount(&mock).await;
        Mock::given(method("POST"))
            .and(path("/api/project/1/p1/end_sync"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock)
            .await;

        let mut srv = server::McpServer::new(mock.uri());
        let args = crate::CliArgs {
            server_url: mock.uri(),
            email: Some("u@t.com".into()),
            password: Some("p".into()),
            install_id: None,
            user_id: None,
            auth_token: None,
        };
        srv.configure(&args).await.unwrap();
        srv.tool_begin_project_sync(&json!({"project_id":"p1"}))
            .await
            .unwrap();
        let r = srv
            .tool_end_project_sync(&json!({"project_id":"p1"}))
            .await
            .unwrap();
        assert!(r.to_string().contains("success"));
    }

    /// Test: unauthenticated calls fail.
    #[tokio::test]
    async fn unauthenticated_calls_fail() {
        let mut srv = server::McpServer::new("http://localhost".into());
        assert!(srv
            .tool_begin_account_sync()
            .await
            .unwrap_err()
            .contains("authenticated"));
    }

    /// Test: missing sync session fails.
    #[tokio::test]
    async fn missing_sync_session_fails() {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/accounts/login"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"userId":1,"auth":"tok","refresh":"ref"})),
            )
            .mount(&mock)
            .await;

        let mut srv = server::McpServer::new(mock.uri());
        let args = crate::CliArgs {
            server_url: mock.uri(),
            email: Some("u@t.com".into()),
            password: Some("p".into()),
            install_id: None,
            user_id: None,
            auth_token: None,
        };
        srv.configure(&args).await.unwrap();
        let r = srv.tool_create_project(&json!({"project_name":"T"})).await;
        assert!(r.is_err());
    }
}
