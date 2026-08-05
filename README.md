# Hammer Editor MCP Server

A [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) server that acts as a full-featured client for the [Hammer Editor](https://github.com/Darkrock-Studios/hammer-editor) synchronization API.

This allows AI assistants (Claude, Cursor, etc.) to **read, write, and manage** Hammer projects, scenes, notes, timeline events, encyclopedia entries, scene drafts, story ideas, and writing activity — exactly like the desktop app does.

## Quick Start

### Docker

```bash
docker run -i ghcr.io/darkrock-studios/hammer-editor-mcp-server:latest \
  -e HAMMER_SERVER_URL=https://your-hammer-server.example.com
```

### From Source

```bash
cargo build --release
HAMMER_SERVER_URL=https://your-hammer-server.example.com ./target/release/hammer-editor-mcp-server
```

## MCP Tools

The server exposes **21 MCP tools**:

### Authentication
| Tool | Description |
|---|---|
| `hammer_login` | Authenticate (email + password) |
| `hammer_set_user_id` | Set the numeric user ID |

### Account Sync
| Tool | Description |
|---|---|
| `hammer_begin_account_sync` | Start account sync → returns project list, sync ID |
| `hammer_end_account_sync` | End account sync |
| `hammer_create_project` | Create a new project |
| `hammer_delete_project` | Delete a project |
| `hammer_rename_project` | Rename a project |
| `hammer_sync_probe` | Fast check: which projects changed? |

### Project Sync
| Tool | Description |
|---|---|
| `hammer_begin_project_sync` | Start project sync → returns sync ID, entity sequence |
| `hammer_end_project_sync` | End project sync |
| `hammer_download_entity` | Download an entity (scene/note/timeline/encyclopedia/draft) |
| `hammer_upload_entity` | Upload or update an entity |
| `hammer_delete_entity` | Delete an entity |
| `hammer_get_project_data` | Get project metadata (author, theme, word goal, tags) |
| `hammer_upload_project_data` | Update project metadata |

### Ideas & Activity
| Tool | Description |
|---|---|
| `hammer_get_ideas_state` | Get story ideas sync state |
| `hammer_download_idea` | Download a story idea |
| `hammer_upload_idea` | Upload/update a story idea |
| `hammer_delete_idea` | Delete a story idea |
| `hammer_get_writing_activity` | Get writing stats (all devices) |

### Utility
| Tool | Description |
|---|---|
| `hammer_entity_schema` | Get JSON schema for an entity type |

## Typical Workflow

```
1. hammer_login(email, password)
2. hammer_begin_account_sync()
   → returns project list + sync_id
3. hammer_begin_project_sync(project_id)
   → returns sync_id + entities to sync
4. hammer_download_entity(project_id, entity_id)  ← for each new/changed entity
5. [AI edits entity content]
6. hammer_upload_entity(project_id, entity_type, entity)
7. hammer_end_project_sync(project_id)
8. hammer_end_account_sync()
```

## Configuration

| Env Var | Default | Description |
|---|---|---|
| `HAMMER_SERVER_URL` | `http://localhost:8080` | Hammer sync server URL |
| `RUST_LOG` | `info` | Log level (error, warn, info, debug, trace) |

## Entity Types

Each entity type has its own JSON schema:

- **Scene** — `{id, scene_type, order, name, path, content, outline, notes, archived, confirmed_references, dismissed_references, tags, created, lastEdited}`
- **Note** — `{id, content, created, tags}`
- **TimelineEvent** — `{id, order, date, content, tags}`
- **EncyclopediaEntry** — `{id, name, entryType, text, tags, image, aliases}`
- **SceneDraft** — `{id, sceneId, created, name, content}`

Use `hammer_entity_schema(entity_type)` to get the full schema at runtime.

## License

MIT