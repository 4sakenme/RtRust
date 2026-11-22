//! Pure-Rust Studio AI Assistant & Model Context Protocol (MCP) Engine for RtRustMobile.
//!
//! - Step 0: Probes & integrates Roblox Official Studio Assistant API via the user's
//!   authenticated `.ROBLOSECURITY` + CSRF `Session`, with automatic fallback to
//!   Free Token MCP providers (Google Gemini 2.5 Flash, Groq, Custom OpenAI-compatible).
//! - Built-in Studio MCP Tool Registry (`get_place_hierarchy`, `read_script`,
//!   `write_script`, `run_luau_command`, `insert_toolbox_asset`, `playtest_and_verify`)
//!   wrapped in `ChangeHistoryService:SetWaypoint()` for full Undo support.
//! - Two-way MCP:
//!   1. Built-in JSON-RPC 2.0 HTTP/SSE MCP Server on `127.0.0.1:28821`
//!   2. External MCP Server Manager (`mcp_servers.json`, HTTP/SSE & stdio)

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, RichText};
use serde_json::{json, Value};

use crate::app::Editor;
use crate::studio_ui::RunType;

const MCP_SERVER_ADDR: &str = "127.0.0.1:28821";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderKind {
    RobloxOfficial,
    GeminiFree,
    GroqFree,
    CustomOpenAi,
}

impl ProviderKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::RobloxOfficial => "Roblox Official",
            Self::GeminiFree => "Gemini Free",
            Self::GroqFree => "Groq Free",
            Self::CustomOpenAi => "Custom MCP/API",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RobloxProbeState {
    NotProbed,
    Probing,
    Available,
    Blocked,
}

#[derive(Clone, Debug)]
pub struct ExternalMcpServer {
    pub name: String,
    pub transport: String, // "http" | "stdio"
    pub endpoint: String,  // URL for http, command for stdio
    pub args: Vec<String>,
    pub enabled: bool,
    pub tools: Vec<McpToolSpec>,
    pub status: String,
}

#[derive(Clone, Debug)]
pub struct McpToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    pub owner_server: Option<String>, // None = built-in Studio tool
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskStatus {
    PendingApproval,
    Running,
    Done,
    Failed,
}

#[derive(Clone, Debug)]
pub struct ChecklistItem {
    pub id: u64,
    pub title: String,
    pub tool_name: String,
    pub args: Value,
    pub status: TaskStatus,
    pub detail: String,
}

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub role: String, // "user" | "assistant" | "system" | "tool"
    pub text: String,
}

struct PendingServerCall {
    tool_name: String,
    args: Value,
    reply: Sender<Result<Value, String>>,
}

struct PendingPlaytest {
    started_at: Instant,
    duration: Duration,
    initial_output_len: usize,
    reply: Option<Sender<Result<Value, String>>>,
    from_agent_loop: bool,
}

enum BgEvent {
    ProbeDone {
        ok: bool,
        message: String,
    },
    LlmReply {
        assistant_text: Option<String>,
        tool_calls: Vec<(String, Value)>,
        prompt_tokens: u64,
        completion_tokens: u64,
        error: Option<String>,
    },
    ExternalToolsDiscovered {
        server_name: String,
        tools: Result<Vec<McpToolSpec>, String>,
    },
}

pub struct McpAssistantState {
    pub open: bool,
    pub show_settings: bool,
    pub show_mcp_servers: bool,

    pub provider: ProviderKind,
    pub roblox_probe: RobloxProbeState,
    pub roblox_probe_note: String,
    pub roblox_custom_endpoint: String,
    pub roblox_custom_header: String,

    pub gemini_api_key: String,
    pub groq_api_key: String,
    pub custom_api_url: String,
    pub custom_api_key: String,
    pub custom_model: String,

    pub auto_run_tools: bool,
    pub server_running: bool,
    pub server_status: String,

    pub prompt_input: String,
    pub messages: Vec<ChatMessage>,
    pub checklist: Vec<ChecklistItem>,
    pub next_task_id: u64,
    pub busy: bool,
    pub total_prompt_tokens: u64,
    pub total_completion_tokens: u64,
    pub ai_undo_waypoints: usize,

    pub external_servers: Vec<ExternalMcpServer>,
    pub new_srv_name: String,
    pub new_srv_transport: String,
    pub new_srv_endpoint: String,
    pub new_srv_args: String,

    // Channels for background HTTP/LLM/MCP server threads
    bg_tx: Sender<BgEvent>,
    bg_rx: Receiver<BgEvent>,
    srv_rx: Option<Receiver<PendingServerCall>>,
    pending_playtest: Option<PendingPlaytest>,
    pending_tool_queue: Vec<(u64, String, Value)>,
    agent_step_budget: usize,
    conversation_history: Vec<Value>,
}

impl Default for McpAssistantState {
    fn default() -> Self {
        let (bg_tx, bg_rx) = mpsc::channel();
        let mut st = Self {
            open: false,
            show_settings: false,
            show_mcp_servers: false,
            provider: ProviderKind::GeminiFree,
            roblox_probe: RobloxProbeState::NotProbed,
            roblox_probe_note: "Roblox Assistant nội bộ dùng endpoint private; mặc định dùng Gemini Free / MCP.".to_owned(),
            roblox_custom_endpoint: String::new(),
            roblox_custom_header: String::new(),
            gemini_api_key: std::env::var("GEMINI_API_KEY").unwrap_or_default(),
            groq_api_key: std::env::var("GROQ_API_KEY").unwrap_or_default(),
            custom_api_url: "https://api.openai.com/v1/chat/completions".to_owned(),
            custom_api_key: String::new(),
            custom_model: "gpt-4o-mini".to_owned(),
            auto_run_tools: true,
            server_running: false,
            server_status: format!("Stopped ({MCP_SERVER_ADDR})"),
            prompt_input: String::new(),
            messages: vec![ChatMessage {
                role: "system".to_owned(),
                text: "Xin chào! Tôi là Studio AI Assistant (MCP). Hãy yêu cầu tôi đọc cây Explorer, viết Script, chạy Luau dựng map/UI, chèn model Toolbox hoặc tự chạy Playtest sửa lỗi."
                    .to_owned(),
            }],
            checklist: Vec::new(),
            next_task_id: 1,
            busy: false,
            total_prompt_tokens: 0,
            total_completion_tokens: 0,
            ai_undo_waypoints: 0,
            external_servers: Vec::new(),
            new_srv_name: String::new(),
            new_srv_transport: "http".to_owned(),
            new_srv_endpoint: "http://127.0.0.1:3000/mcp".to_owned(),
            new_srv_args: String::new(),
            bg_tx,
            bg_rx,
            srv_rx: None,
            pending_playtest: None,
            pending_tool_queue: Vec::new(),
            agent_step_budget: 0,
            conversation_history: Vec::new(),
        };
        st.load_config();
        st
    }
}

static STATE: OnceLock<Mutex<McpAssistantState>> = OnceLock::new();

fn with_state<R>(f: impl FnOnce(&mut McpAssistantState) -> R) -> R {
    let m = STATE.get_or_init(|| Mutex::new(McpAssistantState::default()));
    let mut g = m.lock().expect("mcp_assistant lock");
    f(&mut g)
}

fn config_dir() -> PathBuf {
    let dir = rustudio_core::paths::data().join("rtrustmobile");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn servers_json_path() -> PathBuf {
    config_dir().join("mcp_servers.json")
}

fn assistant_cfg_path() -> PathBuf {
    config_dir().join("mcp_assistant.json")
}

impl McpAssistantState {
    fn load_config(&mut self) {
        if let Ok(raw) = std::fs::read_to_string(assistant_cfg_path()) {
            if let Ok(v) = serde_json::from_str::<Value>(&raw) {
                if let Some(k) = v["gemini_api_key"].as_str() {
                    if !k.is_empty() {
                        self.gemini_api_key = k.to_owned();
                    }
                }
                if let Some(k) = v["groq_api_key"].as_str() {
                    if !k.is_empty() {
                        self.groq_api_key = k.to_owned();
                    }
                }
                if let Some(u) = v["custom_api_url"].as_str() {
                    if !u.is_empty() {
                        self.custom_api_url = u.to_owned();
                    }
                }
                if let Some(k) = v["custom_api_key"].as_str() {
                    self.custom_api_key = k.to_owned();
                }
                if let Some(m) = v["custom_model"].as_str() {
                    if !m.is_empty() {
                        self.custom_model = m.to_owned();
                    }
                }
                if let Some(e) = v["roblox_custom_endpoint"].as_str() {
                    self.roblox_custom_endpoint = e.to_owned();
                }
                if let Some(h) = v["roblox_custom_header"].as_str() {
                    self.roblox_custom_header = h.to_owned();
                }
                if let Some(a) = v["auto_run_tools"].as_bool() {
                    self.auto_run_tools = a;
                }
            }
        }
        if let Ok(raw) = std::fs::read_to_string(servers_json_path()) {
            if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(&raw) {
                if let Some(Value::Object(srvs)) = map.get("mcpServers") {
                    self.external_servers.clear();
                    for (name, cfg) in srvs {
                        let url = cfg["url"].as_str().unwrap_or("");
                        let cmd = cfg["command"].as_str().unwrap_or("");
                        let (transport, endpoint) = if !url.is_empty() {
                            ("http".to_owned(), url.to_owned())
                        } else {
                            ("stdio".to_owned(), cmd.to_owned())
                        };
                        let args = cfg["args"]
                            .as_array()
                            .map(|a| a.iter().filter_map(|x| x.as_str().map(ToOwned::to_owned)).collect())
                            .unwrap_or_default();
                        let enabled = cfg["enabled"].as_bool().unwrap_or(true);
                        self.external_servers.push(ExternalMcpServer {
                            name: name.clone(),
                            transport,
                            endpoint,
                            args,
                            enabled,
                            tools: Vec::new(),
                            status: "Chưa kết nối".to_owned(),
                        });
                    }
                }
            }
        }
    }

    fn save_config(&self) {
        let cfg = json!({
            "gemini_api_key": self.gemini_api_key,
            "groq_api_key": self.groq_api_key,
            "custom_api_url": self.custom_api_url,
            "custom_api_key": self.custom_api_key,
            "custom_model": self.custom_model,
            "roblox_custom_endpoint": self.roblox_custom_endpoint,
            "roblox_custom_header": self.roblox_custom_header,
            "auto_run_tools": self.auto_run_tools,
        });
        let _ = std::fs::write(
            assistant_cfg_path(),
            serde_json::to_string_pretty(&cfg).unwrap_or_default(),
        );

        let mut srvs = serde_json::Map::new();
        for s in &self.external_servers {
            let entry = if s.transport == "http" {
                json!({ "url": s.endpoint, "enabled": s.enabled })
            } else {
                json!({ "command": s.endpoint, "args": s.args, "enabled": s.enabled })
            };
            srvs.insert(s.name.clone(), entry);
        }
        let doc = json!({ "mcpServers": Value::Object(srvs) });
        let _ = std::fs::write(
            servers_json_path(),
            serde_json::to_string_pretty(&doc).unwrap_or_default(),
        );
    }
}

pub fn builtin_tools() -> Vec<McpToolSpec> {
    vec![
        McpToolSpec {
            name: "get_place_hierarchy".to_owned(),
            description: "Read Explorer hierarchy (Name, ClassName, full path) and key Properties of the open Place.".to_owned(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "root_path": { "type": "string", "description": "Optional dot-separated path like Workspace or ReplicatedStorage. Defaults to DataModel root." },
                    "max_depth": { "type": "integer", "description": "Max traversal depth (default 4)." }
                }
            }),
            owner_server: None,
        },
        McpToolSpec {
            name: "read_script".to_owned(),
            description: "Read full source or line range of a Script, LocalScript, or ModuleScript.".to_owned(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Dot-separated path to the script, e.g. ServerScriptService.Main" },
                    "start_line": { "type": "integer" },
                    "end_line": { "type": "integer" }
                },
                "required": ["path"]
            }),
            owner_server: None,
        },
        McpToolSpec {
            name: "write_script".to_owned(),
            description: "Create or overwrite a Script, LocalScript, or ModuleScript at the given parent path (wrapped in Undo waypoint).".to_owned(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "parent_path": { "type": "string", "description": "Parent service/instance path, e.g. ServerScriptService, StarterPlayer.StarterPlayerScripts, ReplicatedStorage" },
                    "name": { "type": "string" },
                    "class_name": { "type": "string", "enum": ["Script", "LocalScript", "ModuleScript"] },
                    "source": { "type": "string" }
                },
                "required": ["parent_path", "name", "class_name", "source"]
            }),
            owner_server: None,
        },
        McpToolSpec {
            name: "run_luau_command".to_owned(),
            description: "Execute Luau code directly in Studio Edit mode (create Parts, ScreenGui, RemoteEvent, edit Properties, SetWaypoint) and return output.".to_owned(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "code": { "type": "string", "description": "Luau source code to execute in Edit mode." },
                    "waypoint": { "type": "string", "description": "Optional Undo waypoint label." }
                },
                "required": ["code"]
            }),
            owner_server: None,
        },
        McpToolSpec {
            name: "insert_toolbox_asset".to_owned(),
            description: "Insert a Roblox Creator Store / Toolbox asset by numeric AssetId into Workspace.".to_owned(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "asset_id": { "type": "integer", "description": "Numeric Roblox Asset ID" }
                },
                "required": ["asset_id"]
            }),
            owner_server: None,
        },
        McpToolSpec {
            name: "playtest_and_verify".to_owned(),
            description: "Start Studio Run/Playtest, collect Output Console logs and runtime errors, then stop and return logs for verification.".to_owned(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "mode": { "type": "string", "enum": ["Run", "Play"] },
                    "duration_sec": { "type": "number", "description": "Seconds to run before collecting Output (default 2.0)." }
                }
            }),
            owner_server: None,
        },
        McpToolSpec {
            name: "read_output".to_owned(),
            description: "Read the latest Output Console lines (errors, warnings, prints) from Studio.".to_owned(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Max recent lines to return (default 50)." }
                }
            }),
            owner_server: None,
        },
    ]
}

fn all_tools_snapshot(st: &McpAssistantState) -> Vec<McpToolSpec> {
    let mut all = builtin_tools();
    for s in &st.external_servers {
        if s.enabled {
            all.extend(s.tools.clone());
        }
    }
    all
}

pub fn toggle_open() {
    with_state(|st| st.open = !st.open);
}

pub fn is_open() -> bool {
    with_state(|st| st.open)
}

/// Step 0: Probe Roblox Official Studio Assistant API using authenticated Session.
pub fn probe_roblox_assistant(ed: &Editor, ctx: &egui::Context) {
    let creds = ed
        .game
        .account
        .with_client(|c| (c.session.cookie(), c.session.profile.clone()));
    let (tx, custom_ep, custom_hdr) = with_state(|st| {
        st.roblox_probe = RobloxProbeState::Probing;
        st.roblox_probe_note = "Đang dò Roblox Studio Assistant API bằng phiên đăng nhập...".to_owned();
        (
            st.bg_tx.clone(),
            st.roblox_custom_endpoint.clone(),
            st.roblox_custom_header.clone(),
        )
    });
    let ctx = ctx.clone();

    std::thread::spawn(move || {
        let Some((cookie, profile)) = creds else {
            let _ = tx.send(BgEvent::ProbeDone {
                ok: false,
                message: "Chưa đăng nhập tài khoản Roblox — chuyển sang chế độ Free Token / MCP dự phòng.".to_owned(),
            });
            ctx.request_repaint();
            return;
        };
        let session = rustudio_account::Session::unchecked(cookie, profile);

        let mut endpoints = Vec::new();
        if !custom_ep.trim().is_empty() {
            endpoints.push(custom_ep.trim().to_owned());
        }
        endpoints.push("https://apis.roblox.com/assistant/v1/conversations".to_owned());
        endpoints.push("https://apis.roblox.com/assistant-api/v1/conversations".to_owned());
        endpoints.push("https://apis.roblox.com/studio-assistant/v1/chat".to_owned());

        let mut extra_headers: Vec<(String, String)> = Vec::new();
        if let Some((k, v)) = custom_hdr.split_once(':') {
            extra_headers.push((k.trim().to_owned(), v.trim().to_owned()));
        }
        let header_refs: Vec<(&str, &str)> = extra_headers
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let probe_body = br#"{"message":"ping","messages":[{"role":"user","content":"ping"}]}"#;
        let mut last_err = String::new();
        for url in endpoints {
            match session.post_csrf_with_timeout(
                &url,
                "application/json",
                probe_body,
                &header_refs,
                Some(Duration::from_secs(6)),
            ) {
                Ok((status, body)) if (200..300).contains(&status) => {
                    let _ = tx.send(BgEvent::ProbeDone {
                        ok: true,
                        message: format!("Roblox Official Assistant sẵn sàng ({status}): {}", &body.chars().take(80).collect::<String>()),
                    });
                    ctx.request_repaint();
                    return;
                }
                Ok((status, _)) => {
                    last_err = format!("HTTP {status} tại {url}");
                }
                Err(e) => {
                    last_err = format!("{e}");
                }
            }
        }

        let _ = tx.send(BgEvent::ProbeDone {
            ok: false,
            message: format!(
                "Roblox Assistant từ chối/chưa mở Client ID ({last_err}). Tự động chuyển sang Gemini Free / MCP."
            ),
        });
        ctx.request_repaint();
    });
}

/// Start built-in HTTP/SSE JSON-RPC 2.0 MCP Server on `127.0.0.1:28821`.
pub fn start_mcp_server(ctx: &egui::Context) {
    let already = with_state(|st| st.server_running);
    if already {
        return;
    }
    let (tx, rx) = mpsc::channel::<PendingServerCall>();
    let listener = match TcpListener::bind(MCP_SERVER_ADDR) {
        Ok(l) => l,
        Err(e) => {
            with_state(|st| {
                st.server_running = false;
                st.server_status = format!("Bind failed: {e}");
            });
            return;
        }
    };
    with_state(|st| {
        st.srv_rx = Some(rx);
        st.server_running = true;
        st.server_status = format!("Listening on http://{MCP_SERVER_ADDR}");
    });
    let ctx = ctx.clone();

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { break };
            let tx = tx.clone();
            let ctx = ctx.clone();
            std::thread::spawn(move || handle_mcp_http(stream, tx, ctx));
        }
    });
}

fn handle_mcp_http(mut stream: TcpStream, tx: Sender<PendingServerCall>, ctx: egui::Context) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let mut reader = BufReader::new(match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    });

    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() {
        return;
    }
    let mut content_len = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() || line.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            if k.eq_ignore_ascii_case("content-length") {
                content_len = v.trim().parse().unwrap_or(0);
            }
        }
    }

    if request_line.starts_with("GET ") {
        let info = json!({
            "name": "rtrust-mobile-studio-mcp",
            "version": "1.0.0",
            "protocol": "jsonrpc-2.0",
            "tools": builtin_tools().iter().map(|t| &t.name).collect::<Vec<_>>(),
        });
        let body = info.to_string();
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(resp.as_bytes());
        return;
    }

    let mut body_buf = vec![0u8; content_len.min(4 * 1024 * 1024)];
    if content_len > 0 && reader.read_exact(&mut body_buf).is_err() {
        return;
    }
    let req: Value = serde_json::from_slice(&body_buf).unwrap_or(Value::Null);
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let method = req["method"].as_str().unwrap_or("");

    let reply_json = match method {
        "initialize" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "serverInfo": { "name": "RtRustMobile-Studio-MCP", "version": "0.3.2" },
                "capabilities": { "tools": {} }
            }
        }),
        "tools/list" => {
            let tools: Vec<Value> = builtin_tools()
                .into_iter()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "inputSchema": t.parameters,
                    })
                })
                .collect();
            json!({ "jsonrpc": "2.0", "id": id, "result": { "tools": tools } })
        }
        "tools/call" => {
            let name = req["params"]["name"].as_str().unwrap_or("").to_owned();
            let args = req["params"].get("arguments").cloned().unwrap_or(json!({}));
            let (rtx, rrx) = mpsc::channel();
            if tx.send(PendingServerCall { tool_name: name, args, reply: rtx }).is_ok() {
                ctx.request_repaint();
                match rrx.recv_timeout(Duration::from_secs(15)) {
                    Ok(Ok(val)) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": { "content": [{ "type": "text", "text": val.to_string() }] }
                    }),
                    Ok(Err(e)) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32000, "message": e }
                    }),
                    Err(_) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32001, "message": "Studio tool execution timed out" }
                    }),
                }
            } else {
                json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32002, "message": "Editor closed" } })
            }
        }
        _ => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32601, "message": format!("Method not found: {method}") }
        }),
    };

    let out = reply_json.to_string();
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\n\r\n{}",
        out.len(),
        out
    );
    let _ = stream.write_all(resp.as_bytes());
}

/// Discover `tools/list` from an external MCP server (HTTP JSON-RPC or stdio).
pub fn discover_external_server(server: ExternalMcpServer, ctx: &egui::Context) {
    let tx = with_state(|st| st.bg_tx.clone());
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {}
        });
        let res = call_external_jsonrpc(&server, &payload).map(|resp| {
            resp["result"]["tools"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|t| {
                            let name = t["name"].as_str()?.to_owned();
                            Some(McpToolSpec {
                                name,
                                description: t["description"].as_str().unwrap_or("").to_owned(),
                                parameters: t.get("inputSchema").cloned().unwrap_or(json!({"type":"object"})),
                                owner_server: Some(server.name.clone()),
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        });
        let _ = tx.send(BgEvent::ExternalToolsDiscovered {
            server_name: server.name,
            tools: res,
        });
        ctx.request_repaint();
    });
}

fn call_external_jsonrpc(server: &ExternalMcpServer, payload: &Value) -> Result<Value, String> {
    if server.transport == "http" {
        let mut resp = ureq::post(&server.endpoint)
            .header("Content-Type", "application/json")
            .send(payload.to_string().as_bytes())
            .map_err(|e| format!("MCP HTTP error: {e}"))?;
        let text = resp
            .body_mut()
            .read_to_string()
            .map_err(|e| format!("MCP read error: {e}"))?;
        serde_json::from_str(&text).map_err(|e| format!("MCP JSON error: {e}"))
    } else {
        let mut child = Command::new(&server.endpoint)
            .args(&server.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Spawn `{}` failed: {e}", server.endpoint))?;
        if let Some(mut stdin) = child.stdin.take() {
            let init = json!({
                "jsonrpc": "2.0",
                "id": 0,
                "method": "initialize",
                "params": { "protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": { "name": "RtRustMobile", "version": "0.3.2" } }
            });
            let _ = writeln!(stdin, "{init}");
            let _ = writeln!(stdin, "{payload}");
            let _ = stdin.flush();
        }
        let stdout = child.stdout.take().ok_or("No stdout")?;
        let reader = BufReader::new(stdout);
        let mut last_match = None;
        for line in reader.lines().map_while(Result::ok).take(16) {
            if let Ok(v) = serde_json::from_str::<Value>(&line) {
                if v.get("id") == Some(&payload["id"]) {
                    last_match = Some(v);
                    break;
                }
            }
        }
        let _ = child.kill();
        last_match.ok_or_else(|| "No JSON-RPC response from stdio MCP".to_owned())
    }
}

/// Main-thread tick: drains background LLM/Probe/Server events & runs pending tools/playtest.
pub fn tick(ed: &mut Editor, ctx: &egui::Context) {
    // 1. Check active playtest_and_verify completion
    let finished_pt = with_state(|st| {
        if let Some(pt) = st.pending_playtest.as_ref() {
            if pt.started_at.elapsed() >= pt.duration {
                return st.pending_playtest.take();
            }
        }
        None
    });
    if let Some(mut pt) = finished_pt {
        let logs: Vec<String> = ed
            .game
            .log
            .iter()
            .skip(pt.initial_output_len)
            .map(|e| format!("[{:?}] {}", e.level, e.text))
            .collect();
        ed.stop_play();
        let has_errors = logs.iter().any(|l| l.contains("[Error]"));
        let res = json!({
            "has_errors": has_errors,
            "output_count": logs.len(),
            "logs": logs,
        });
        if let Some(reply) = pt.reply.take() {
            let _ = reply.send(Ok(res.clone()));
        }
        if pt.from_agent_loop {
            finish_tool_in_agent_loop(ed, ctx, "playtest_and_verify", Ok(res));
        }
    }

    // 2. Drain background events
    let events: Vec<BgEvent> = with_state(|st| {
        let mut out = Vec::new();
        while let Ok(ev) = st.bg_rx.try_recv() {
            out.push(ev);
        }
        out
    });

    for ev in events {
        match ev {
            BgEvent::ProbeDone { ok, message } => with_state(|st| {
                st.roblox_probe = if ok {
                    RobloxProbeState::Available
                } else {
                    RobloxProbeState::Blocked
                };
                st.roblox_probe_note = message;
                if ok {
                    st.provider = ProviderKind::RobloxOfficial;
                } else if st.provider == ProviderKind::RobloxOfficial {
                    st.provider = if !st.gemini_api_key.is_empty() {
                        ProviderKind::GeminiFree
                    } else if !st.groq_api_key.is_empty() {
                        ProviderKind::GroqFree
                    } else {
                        ProviderKind::CustomOpenAi
                    };
                }
            }),
            BgEvent::ExternalToolsDiscovered { server_name, tools } => with_state(|st| {
                if let Some(srv) = st.external_servers.iter_mut().find(|s| s.name == server_name) {
                    match tools {
                        Ok(list) => {
                            srv.status = format!("{} tools", list.len());
                            srv.tools = list;
                        }
                        Err(e) => {
                            srv.status = format!("Error: {e}");
                        }
                    }
                }
            }),
            BgEvent::LlmReply {
                assistant_text,
                tool_calls,
                prompt_tokens,
                completion_tokens,
                error,
            } => {
                with_state(|st| {
                    st.busy = false;
                    st.total_prompt_tokens += prompt_tokens;
                    st.total_completion_tokens += completion_tokens;
                    if let Some(err) = error {
                        st.messages.push(ChatMessage {
                            role: "system".to_owned(),
                            text: format!("❌ {err}"),
                        });
                        return;
                    }
                    if let Some(txt) = assistant_text.as_ref().filter(|s| !s.trim().is_empty()) {
                        st.messages.push(ChatMessage {
                            role: "assistant".to_owned(),
                            text: txt.clone(),
                        });
                        st.conversation_history
                            .push(json!({ "role": "assistant", "content": txt }));
                    }
                    for (tname, targs) in tool_calls {
                        let id = st.next_task_id;
                        st.next_task_id += 1;
                        let status = if st.auto_run_tools {
                            TaskStatus::Running
                        } else {
                            TaskStatus::PendingApproval
                        };
                        st.checklist.push(ChecklistItem {
                            id,
                            title: format!("Tool: {tname}"),
                            tool_name: tname.clone(),
                            args: targs.clone(),
                            status,
                            detail: String::new(),
                        });
                        if st.auto_run_tools {
                            st.pending_tool_queue.push((id, tname, targs));
                        }
                    }
                });
            }
        }
    }

    // 3. Execute queued agent tool calls (1 per frame so UI stays responsive)
    let next_agent_tool = with_state(|st| {
        if st.pending_playtest.is_some() || st.pending_tool_queue.is_empty() {
            None
        } else {
            Some(st.pending_tool_queue.remove(0))
        }
    });
    if let Some((task_id, tool_name, args)) = next_agent_tool {
        if tool_name == "playtest_and_verify" {
            start_playtest_tool(ed, args, None, true);
        } else {
            let res = execute_any_tool(ed, &tool_name, &args);
            with_state(|st| {
                if let Some(item) = st.checklist.iter_mut().find(|c| c.id == task_id) {
                    match &res {
                        Ok(v) => {
                            item.status = TaskStatus::Done;
                            item.detail = v.to_string().chars().take(180).collect();
                        }
                        Err(e) => {
                            item.status = TaskStatus::Failed;
                            item.detail = e.clone();
                        }
                    }
                }
            });
            finish_tool_in_agent_loop(ed, ctx, &tool_name, res);
        }
    }

    // 4. Drain external HTTP MCP Server requests (`127.0.0.1:28821`)
    let server_calls: Vec<PendingServerCall> = with_state(|st| {
        let mut out = Vec::new();
        if let Some(rx) = st.srv_rx.as_ref() {
            while let Ok(call) = rx.try_recv() {
                out.push(call);
            }
        }
        out
    });
    for call in server_calls {
        if call.tool_name == "playtest_and_verify" {
            start_playtest_tool(ed, call.args, Some(call.reply), false);
        } else {
            let res = execute_any_tool(ed, &call.tool_name, &call.args);
            let _ = call.reply.send(res);
        }
    }
}

fn finish_tool_in_agent_loop(
    ed: &Editor,
    ctx: &egui::Context,
    tool_name: &str,
    res: Result<Value, String>,
) {
    let summary = match &res {
        Ok(v) => v.to_string(),
        Err(e) => format!("ERROR: {e}"),
    };
    let should_continue = with_state(|st| {
        st.conversation_history.push(json!({
            "role": "user",
            "content": format!("[Tool Result for `{tool_name}`]:\n{summary}")
        }));
        if st.pending_tool_queue.is_empty() && st.agent_step_budget > 0 {
            st.agent_step_budget -= 1;
            true
        } else {
            false
        }
    });
    if should_continue {
        dispatch_llm_turn(ed, ctx);
    }
}

fn start_playtest_tool(
    ed: &mut Editor,
    args: Value,
    reply: Option<Sender<Result<Value, String>>>,
    from_agent_loop: bool,
) {
    let mode = args["mode"].as_str().unwrap_or("Run");
    let dur = args["duration_sec"].as_f64().unwrap_or(2.0).clamp(0.5, 10.0);
    let initial_len = ed.game.log.len();
    let rt = if mode.eq_ignore_ascii_case("Play") || mode.eq_ignore_ascii_case("Test") {
        RunType::Test
    } else {
        RunType::Run
    };
    ed.start_run(rt);
    with_state(|st| {
        st.pending_playtest = Some(PendingPlaytest {
            started_at: Instant::now(),
            duration: Duration::from_secs_f64(dur),
            initial_output_len: initial_len,
            reply,
            from_agent_loop,
        });
    });
}

fn execute_any_tool(ed: &mut Editor, tool_name: &str, args: &Value) -> Result<Value, String> {
    let ext = with_state(|st| {
        st.external_servers
            .iter()
            .find(|s| s.enabled && s.tools.iter().any(|t| t.name == tool_name))
            .cloned()
    });
    if let Some(srv) = ext {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 99,
            "method": "tools/call",
            "params": { "name": tool_name, "arguments": args }
        });
        let resp = call_external_jsonrpc(&srv, &payload)?;
        return Ok(resp.get("result").cloned().unwrap_or(resp));
    }

    match tool_name {
        "get_place_hierarchy" => tool_get_hierarchy(ed, args),
        "read_script" => tool_read_script(ed, args),
        "write_script" => tool_write_script(ed, args),
        "run_luau_command" => tool_run_luau(ed, args),
        "insert_toolbox_asset" => tool_insert_toolbox(ed, args),
        "read_output" => tool_read_output(ed, args),
        _ => Err(format!("Unknown MCP tool: {tool_name}")),
    }
}

fn tool_read_output(ed: &Editor, args: &Value) -> Result<Value, String> {
    let limit = args["limit"].as_u64().unwrap_or(50).min(200) as usize;
    let total = ed.game.log.len();
    let skip = total.saturating_sub(limit);
    let lines: Vec<String> = ed
        .game
        .log
        .iter()
        .skip(skip)
        .map(|l| format!("[{:?}] {}", l.level, l.text))
        .collect();
    Ok(json!({ "total": total, "lines": lines }))
}

fn resolve_path(doc: &rustudio_core::doc::Document, path: &str) -> Option<rbx_types::Ref> {
    let mut cur = doc.dom.root_ref();
    let trimmed = path.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("game") {
        return Some(cur);
    }
    for seg in trimmed.split('.') {
        let seg = seg.trim();
        if seg.eq_ignore_ascii_case("game") {
            cur = doc.dom.root_ref();
            continue;
        }
        let inst = doc.dom.get_by_ref(cur)?;
        let next = inst.children().iter().copied().find(|&c| {
            doc.dom
                .get_by_ref(c)
                .is_some_and(|ci| ci.name.eq_ignore_ascii_case(seg))
        })?;
        cur = next;
    }
    Some(cur)
}

fn tool_get_hierarchy(ed: &Editor, args: &Value) -> Result<Value, String> {
    let doc = &ed.game.doc;
    let max_depth = args["max_depth"].as_u64().unwrap_or(4).min(8) as usize;
    let root = match args["root_path"].as_str() {
        Some(p) if !p.trim().is_empty() => {
            resolve_path(doc, p).ok_or_else(|| format!("Path not found: {p}"))?
        }
        _ => doc.dom.root_ref(),
    };

    fn walk(
        doc: &rustudio_core::doc::Document,
        r: rbx_types::Ref,
        depth: usize,
        max_depth: usize,
        count: &mut usize,
    ) -> Value {
        let Some(inst) = doc.dom.get_by_ref(r) else {
            return Value::Null;
        };
        *count += 1;
        let mut props = serde_json::Map::new();
        for key in ["Position", "Size", "Anchored", "CanCollide", "Enabled", "Text", "Value"] {
            if let Some(v) = inst.properties.get(&rbx_dom_weak::ustr(key)) {
                props.insert(key.to_owned(), Value::String(format!("{v:?}")));
            }
        }
        let children: Vec<Value> = if depth < max_depth && *count < 250 {
            inst.children()
                .iter()
                .copied()
                .map(|c| walk(doc, c, depth + 1, max_depth, count))
                .collect()
        } else {
            Vec::new()
        };
        json!({
            "name": inst.name,
            "class": inst.class,
            "properties": props,
            "children": children,
        })
    }

    let mut count = 0;
    Ok(walk(doc, root, 0, max_depth, &mut count))
}

fn tool_read_script(ed: &Editor, args: &Value) -> Result<Value, String> {
    let path = args["path"].as_str().ok_or("Missing `path`")?;
    let r = resolve_path(&ed.game.doc, path).ok_or_else(|| format!("Script not found: {path}"))?;
    let src = match ed.game.doc.prop(r, "Source") {
        Some(rbx_types::Variant::String(s)) => s,
        _ => return Err(format!("Instance `{path}` has no Source property")),
    };
    let lines: Vec<&str> = src.lines().collect();
    let start = args["start_line"].as_u64().unwrap_or(1).max(1) as usize;
    let end = args["end_line"].as_u64().unwrap_or(lines.len() as u64).min(lines.len() as u64) as usize;
    let slice = if start <= end && start <= lines.len() {
        lines[start - 1..end].join("\n")
    } else {
        String::new()
    };
    Ok(json!({ "path": path, "total_lines": lines.len(), "source": slice }))
}

fn tool_write_script(ed: &mut Editor, args: &Value) -> Result<Value, String> {
    let parent_path = args["parent_path"].as_str().ok_or("Missing `parent_path`")?;
    let name = args["name"].as_str().ok_or("Missing `name`")?;
    let class_name = args["class_name"].as_str().unwrap_or("Script");
    let source = args["source"].as_str().ok_or("Missing `source`")?;

    let escaped_src = format!("{source:?}");
    let luau = format!(
        r#"
local chs = game:GetService("ChangeHistoryService")
chs:SetWaypoint("AI: write_script {name}")
local cur = game
for seg in string.gmatch("{parent_path}", "[^%.]+") do
    if seg ~= "game" then
        cur = cur:FindFirstChild(seg) or game:GetService(seg)
    end
end
assert(cur, "Parent path not found: {parent_path}")
local s = cur:FindFirstChild("{name}")
if not s then
    s = Instance.new("{class_name}")
    s.Name = "{name}"
    s.Parent = cur
end
s.Source = {escaped_src}
chs:SetWaypoint("AI: write_script {name} done")
print("Wrote " .. s:GetFullName())
"#
    );
    let out = run_luau_in_edit(ed, &luau)?;
    with_state(|st| st.ai_undo_waypoints += 1);
    Ok(json!({ "status": "ok", "output": out }))
}

fn tool_run_luau(ed: &mut Editor, args: &Value) -> Result<Value, String> {
    let code = args["code"].as_str().ok_or("Missing `code`")?;
    let wp = args["waypoint"].as_str().unwrap_or("AI: run_luau_command");
    let wrapped = format!(
        "local __chs = game:GetService(\"ChangeHistoryService\")\n__chs:SetWaypoint({wp:?})\n{code}\n__chs:SetWaypoint(\"{wp} done\")"
    );
    let out = run_luau_in_edit(ed, &wrapped)?;
    with_state(|st| st.ai_undo_waypoints += 1);
    Ok(json!({ "status": "ok", "output": out }))
}

fn tool_insert_toolbox(ed: &mut Editor, args: &Value) -> Result<Value, String> {
    let id = args["asset_id"].as_u64().ok_or("Missing `asset_id`")?;
    match ed.toolbox_insert(id, crate::toolbox::insert::Target::Click) {
        Some(job) => {
            with_state(|st| st.ai_undo_waypoints += 1);
            Ok(json!({ "status": "queued", "asset_id": id, "job": job }))
        }
        None => Err(format!("Failed to queue Toolbox insert for {id}")),
    }
}

fn run_luau_in_edit(ed: &mut Editor, code: &str) -> Result<Vec<String>, String> {
    let res = ed.plugins.with_edit(&mut ed.game.doc, |rt| {
        let exec = rt.lua.load(code).set_name("=mcp_assistant").exec();
        let mut out = Vec::new();
        if let Some(mut w) = rt.lua.app_data_mut::<rustudio_engine::runtime::World>() {
            for (lvl, msg) in std::mem::take(&mut w.output) {
                out.push(format!("[{lvl:?}] {msg}"));
            }
        }
        match exec {
            Ok(()) => Ok(out),
            Err(e) => Err(format!("{e} (output: {})", out.join(" | "))),
        }
    });
    match res {
        Some(inner) => inner,
        None => Err("Edit Luau runtime unavailable".to_owned()),
    }
}

fn dispatch_llm_turn(ed: &Editor, ctx: &egui::Context) {
    let creds = ed
        .game
        .account
        .with_client(|c| (c.session.cookie(), c.session.profile.clone()));
    let (
        tx,
        provider,
        gemini_key,
        groq_key,
        custom_url,
        custom_key,
        custom_model,
        roblox_ep,
        roblox_hdr,
        history,
        tools,
    ) = with_state(|st| {
        st.busy = true;
        (
            st.bg_tx.clone(),
            st.provider,
            st.gemini_api_key.clone(),
            st.groq_api_key.clone(),
            st.custom_api_url.clone(),
            st.custom_api_key.clone(),
            st.custom_model.clone(),
            st.roblox_custom_endpoint.clone(),
            st.roblox_custom_header.clone(),
            st.conversation_history.clone(),
            all_tools_snapshot(st),
        )
    });
    let ctx = ctx.clone();

    std::thread::spawn(move || {
        let system_prompt = "You are RtRustMobile Studio AI Assistant with MCP tools. \
            To inspect or edit the open Roblox Place, call the provided tools (`get_place_hierarchy`, \
            `read_script`, `write_script`, `run_luau_command`, `insert_toolbox_asset`, `playtest_and_verify`). \
            Respond concisely in the user's language.";

        let result = match provider {
            ProviderKind::RobloxOfficial => {
                let session = creds.map(|(c, p)| rustudio_account::Session::unchecked(c, p));
                call_roblox_official(session, &roblox_ep, &roblox_hdr, system_prompt, &history, &tools)
            }
            ProviderKind::GeminiFree => {
                call_gemini(&gemini_key, system_prompt, &history, &tools)
            }
            ProviderKind::GroqFree => call_openai_compat(
                "https://api.groq.com/openai/v1/chat/completions",
                &groq_key,
                "llama-3.3-70b-versatile",
                system_prompt,
                &history,
                &tools,
            ),
            ProviderKind::CustomOpenAi => call_openai_compat(
                &custom_url,
                &custom_key,
                &custom_model,
                system_prompt,
                &history,
                &tools,
            ),
        };

        match result {
            Ok((text, calls, pt, ct)) => {
                let _ = tx.send(BgEvent::LlmReply {
                    assistant_text: text,
                    tool_calls: calls,
                    prompt_tokens: pt,
                    completion_tokens: ct,
                    error: None,
                });
            }
            Err(e) => {
                let _ = tx.send(BgEvent::LlmReply {
                    assistant_text: None,
                    tool_calls: Vec::new(),
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    error: Some(e),
                });
            }
        }
        ctx.request_repaint();
    });
}

type LlmOutcome = Result<(Option<String>, Vec<(String, Value)>, u64, u64), String>;

fn call_roblox_official(
    session: Option<rustudio_account::Session>,
    custom_ep: &str,
    custom_hdr: &str,
    system_prompt: &str,
    history: &[Value],
    _tools: &[McpToolSpec],
) -> LlmOutcome {
    let session = session.ok_or("Chưa đăng nhập Roblox Account")?;
    let url = if custom_ep.trim().is_empty() {
        "https://apis.roblox.com/studio-assistant/v1/chat"
    } else {
        custom_ep.trim()
    };
    let mut msgs = vec![json!({ "role": "system", "content": system_prompt })];
    msgs.extend_from_slice(history);
    let body = json!({ "messages": msgs }).to_string();
    let mut extra = Vec::new();
    if let Some((k, v)) = custom_hdr.split_once(':') {
        extra.push((k.trim(), v.trim()));
    }
    let (status, text) = session
        .post_csrf_with_timeout(
            url,
            "application/json",
            body.as_bytes(),
            &extra,
            Some(Duration::from_secs(20)),
        )
        .map_err(|e| format!("Roblox Assistant error: {e}"))?;
    if !(200..300).contains(&status) {
        return Err(format!("Roblox Assistant HTTP {status}: {text}"));
    }
    let v: Value = serde_json::from_str(&text).unwrap_or_else(|_| json!({ "reply": text }));
    let reply = v["reply"]
        .as_str()
        .or_else(|| v["message"]["content"].as_str())
        .or_else(|| v["choices"][0]["message"]["content"].as_str())
        .map(ToOwned::to_owned);
    Ok((reply, Vec::new(), 0, 0))
}

fn call_gemini(
    api_key: &str,
    system_prompt: &str,
    history: &[Value],
    tools: &[McpToolSpec],
) -> LlmOutcome {
    if api_key.trim().is_empty() {
        return Err("Chưa có Gemini API Key. Bấm '⚙ Provider' để dán key miễn phí (aistudio.google.com), hoặc kết nối Antigravity/Claude/Cursor tới MCP Server: http://127.0.0.1:28821".to_owned());
    }
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}",
        api_key.trim()
    );
    let contents: Vec<Value> = history
        .iter()
        .map(|m| {
            let role = if m["role"] == "assistant" { "model" } else { "user" };
            json!({
                "role": role,
                "parts": [{ "text": m["content"].as_str().unwrap_or("") }]
            })
        })
        .collect();
    let decls: Vec<Value> = tools
        .iter()
        .map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "parameters": t.parameters,
            })
        })
        .collect();
    let body = json!({
        "systemInstruction": { "parts": [{ "text": system_prompt }] },
        "contents": contents,
        "tools": [{ "functionDeclarations": decls }]
    });
    let mut resp = ureq::post(&url)
        .header("Content-Type", "application/json")
        .send(body.to_string().as_bytes())
        .map_err(|e| format!("Gemini HTTP error: {e}"))?;
    let raw = resp.body_mut().read_to_string().map_err(|e| format!("Read error: {e}"))?;
    let v: Value = serde_json::from_str(&raw).map_err(|e| format!("JSON error: {e}"))?;

    let mut text_out = String::new();
    let mut calls = Vec::new();
    if let Some(parts) = v["candidates"][0]["content"]["parts"].as_array() {
        for p in parts {
            if let Some(t) = p["text"].as_str() {
                text_out.push_str(t);
            }
            if let Some(fc) = p.get("functionCall") {
                if let Some(name) = fc["name"].as_str() {
                    calls.push((name.to_owned(), fc.get("args").cloned().unwrap_or(json!({}))));
                }
            }
        }
    }
    let pt = v["usageMetadata"]["promptTokenCount"].as_u64().unwrap_or(0);
    let ct = v["usageMetadata"]["candidatesTokenCount"].as_u64().unwrap_or(0);
    Ok(((!text_out.is_empty()).then_some(text_out), calls, pt, ct))
}

fn call_openai_compat(
    url: &str,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    history: &[Value],
    tools: &[McpToolSpec],
) -> LlmOutcome {
    if api_key.trim().is_empty() {
        return Err("Chưa nhập API Key cho provider hiện tại.".to_owned());
    }
    let mut messages = vec![json!({ "role": "system", "content": system_prompt })];
    messages.extend_from_slice(history);
    let tool_defs: Vec<Value> = tools
        .iter()
        .map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.parameters,
                }
            })
        })
        .collect();
    let body = json!({
        "model": model,
        "messages": messages,
        "tools": tool_defs,
    });
    let mut resp = ureq::post(url)
        .header("Content-Type", "application/json")
        .header("Authorization", &format!("Bearer {}", api_key.trim()))
        .send(body.to_string().as_bytes())
        .map_err(|e| format!("API HTTP error: {e}"))?;
    let raw = resp.body_mut().read_to_string().map_err(|e| format!("Read error: {e}"))?;
    let v: Value = serde_json::from_str(&raw).map_err(|e| format!("JSON error: {e}"))?;
    let msg = &v["choices"][0]["message"];
    let text = msg["content"].as_str().map(ToOwned::to_owned);
    let mut calls = Vec::new();
    if let Some(arr) = msg["tool_calls"].as_array() {
        for c in arr {
            let name = c["function"]["name"].as_str().unwrap_or("");
            let args_str = c["function"]["arguments"].as_str().unwrap_or("{}");
            let args = serde_json::from_str(args_str).unwrap_or(json!({}));
            if !name.is_empty() {
                calls.push((name.to_owned(), args));
            }
        }
    }
    let pt = v["usage"]["prompt_tokens"].as_u64().unwrap_or(0);
    let ct = v["usage"]["completion_tokens"].as_u64().unwrap_or(0);
    Ok((text, calls, pt, ct))
}

/// Mobile-friendly Agentic Studio Assistant & MCP window.
pub fn ui(ed: &mut Editor, ctx: &egui::Context) {
    tick(ed, ctx);

    let open = with_state(|st| st.open);
    if !open {
        return;
    }

    let screen = ctx.content_rect();
    let is_portrait = screen.width() < screen.height();
    let default_size = if is_portrait {
        egui::vec2((screen.width() - 16.0).max(280.0), (screen.height() * 0.68).max(320.0))
    } else {
        egui::vec2(440.0, 560.0)
    };

    let mut win_open = true;
    let mut send_prompt = false;
    let mut do_probe = false;
    let mut do_undo_all = false;
    let mut do_start_srv = false;
    let mut approve_ids: Vec<(u64, String, Value)> = Vec::new();

    egui::Window::new("🤖 Studio AI Assistant (MCP)")
        .id(egui::Id::new("rtrust_mcp_assistant_win"))
        .open(&mut win_open)
        .resizable(true)
        .default_size(default_size)
        .show(ctx, |ui| {
            with_state(|st| {
                // Header status bar
                ui.horizontal_wrapped(|ui| {
                    let badge_col = match st.provider {
                        ProviderKind::RobloxOfficial => Color32::from_rgb(100, 220, 140),
                        ProviderKind::GeminiFree => Color32::from_rgb(110, 185, 255),
                        ProviderKind::GroqFree => Color32::from_rgb(255, 180, 95),
                        ProviderKind::CustomOpenAi => Color32::from_rgb(200, 160, 255),
                    };
                    ui.label(RichText::new(format!("● {}", st.provider.label())).strong().color(badge_col));
                    ui.label(
                        RichText::new(format!(
                            "Tokens: {} in / {} out",
                            st.total_prompt_tokens, st.total_completion_tokens
                        ))
                        .small()
                        .color(Color32::from_gray(170)),
                    );
                });

                ui.horizontal_wrapped(|ui| {
                    if ui.selectable_label(st.auto_run_tools, "⚡ Auto-Run Tools").clicked() {
                        st.auto_run_tools = !st.auto_run_tools;
                        st.save_config();
                    }
                    if st.ai_undo_waypoints > 0
                        && ui
                            .button(RichText::new(format!("↩ Undo AI ({})", st.ai_undo_waypoints)).color(Color32::from_rgb(255, 140, 140)))
                            .clicked()
                    {
                        do_undo_all = true;
                    }
                    if ui.selectable_label(st.show_settings, "⚙ Provider").clicked() {
                        st.show_settings = !st.show_settings;
                    }
                    if ui.selectable_label(st.show_mcp_servers, "🔌 MCP Servers").clicked() {
                        st.show_mcp_servers = !st.show_mcp_servers;
                    }
                });

                if st.show_settings {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for p in [
                                ProviderKind::RobloxOfficial,
                                ProviderKind::GeminiFree,
                                ProviderKind::GroqFree,
                                ProviderKind::CustomOpenAi,
                            ] {
                                if ui.selectable_label(st.provider == p, p.label()).clicked() {
                                    st.provider = p;
                                }
                            }
                        });
                        ui.label(RichText::new(&st.roblox_probe_note).small().color(Color32::from_gray(180)));
                        if ui.button("🔍 Probe Roblox Official Assistant").clicked() {
                            do_probe = true;
                        }
                        match st.provider {
                            ProviderKind::RobloxOfficial => {
                                ui.horizontal(|ui| {
                                    ui.label("Endpoint:");
                                    ui.text_edit_singleline(&mut st.roblox_custom_endpoint);
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Header:");
                                    ui.text_edit_singleline(&mut st.roblox_custom_header);
                                });
                            }
                            ProviderKind::GeminiFree => {
                                ui.horizontal(|ui| {
                                    ui.label("Gemini Key:");
                                    ui.add(egui::TextEdit::singleline(&mut st.gemini_api_key).password(true));
                                });
                            }
                            ProviderKind::GroqFree => {
                                ui.horizontal(|ui| {
                                    ui.label("Groq Key:");
                                    ui.add(egui::TextEdit::singleline(&mut st.groq_api_key).password(true));
                                });
                            }
                            ProviderKind::CustomOpenAi => {
                                ui.horizontal(|ui| {
                                    ui.label("URL:");
                                    ui.text_edit_singleline(&mut st.custom_api_url);
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Model:");
                                    ui.text_edit_singleline(&mut st.custom_model);
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Key:");
                                    ui.add(egui::TextEdit::singleline(&mut st.custom_api_key).password(true));
                                });
                            }
                        }
                        if ui.button("💾 Save Provider Config").clicked() {
                            st.save_config();
                        }
                    });
                }

                if st.show_mcp_servers {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("Agent MCP Server: {}", st.server_status)).small());
                            if !st.server_running && ui.small_button("▶ Start :28821").clicked() {
                                do_start_srv = true;
                            }
                        });
                        ui.label(
                            RichText::new("RuStudio 0.3.3 Official MCP: bật nút 'MCP' trên Top Bar (127.0.0.1:44755 + Bearer token)")
                                .small()
                                .color(Color32::from_rgb(130, 210, 255)),
                        );
                        ui.separator();
                        ui.label(RichText::new("External MCP Servers (mcp_servers.json)").strong().small());
                        let mut to_discover: Option<ExternalMcpServer> = None;
                        for s in &mut st.external_servers {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut s.enabled, &s.name);
                                ui.label(RichText::new(format!("[{}] {}", s.transport, s.status)).small());
                                if ui.small_button("↻ List").clicked() {
                                    to_discover = Some(s.clone());
                                }
                            });
                        }
                        if let Some(srv) = to_discover {
                            discover_external_server(srv, ctx);
                        }
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut st.new_srv_name).desired_width(80.0).hint_text("Name"));
                            ui.add(egui::TextEdit::singleline(&mut st.new_srv_endpoint).desired_width(140.0).hint_text("URL or cmd"));
                            if ui.small_button("+ HTTP").clicked() && !st.new_srv_name.is_empty() {
                                let srv = ExternalMcpServer {
                                    name: st.new_srv_name.clone(),
                                    transport: "http".to_owned(),
                                    endpoint: st.new_srv_endpoint.clone(),
                                    args: Vec::new(),
                                    enabled: true,
                                    tools: Vec::new(),
                                    status: "Probing...".to_owned(),
                                };
                                st.external_servers.push(srv.clone());
                                st.save_config();
                                discover_external_server(srv, ctx);
                            }
                        });
                    });
                }

                // Real-time Task Checklist
                if !st.checklist.is_empty() {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.label(RichText::new("📋 Task Checklist").strong().small());
                        for item in &mut st.checklist {
                            ui.horizontal(|ui| {
                                let icon = match item.status {
                                    TaskStatus::PendingApproval => "⏸",
                                    TaskStatus::Running => "⏳",
                                    TaskStatus::Done => "✅",
                                    TaskStatus::Failed => "❌",
                                };
                                ui.label(format!("{icon} {}", item.title));
                                if item.status == TaskStatus::PendingApproval && ui.small_button("Approve").clicked() {
                                    item.status = TaskStatus::Running;
                                    approve_ids.push((item.id, item.tool_name.clone(), item.args.clone()));
                                }
                            });
                        }
                    });
                }

                ui.separator();
                let chat_h = (ui.available_height() - 48.0).max(120.0);
                egui::ScrollArea::vertical()
                    .id_salt("mcp_chat_scroll")
                    .max_height(chat_h)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for m in &st.messages {
                            let color = match m.role.as_str() {
                                "user" => Color32::from_rgb(130, 210, 255),
                                "assistant" => Color32::from_rgb(235, 240, 245),
                                _ => Color32::from_gray(175),
                            };
                            let prefix = if m.role == "user" { "You: " } else { "" };
                            ui.label(RichText::new(format!("{prefix}{}", m.text)).color(color));
                            ui.add_space(4.0);
                        }
                        if st.busy {
                            ui.label(RichText::new("⏳ Đang suy nghĩ / gọi MCP tool...").italics().color(Color32::from_rgb(255, 210, 120)));
                        }
                    });

                ui.separator();
                ui.horizontal(|ui| {
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut st.prompt_input)
                            .desired_width((ui.available_width() - 72.0).max(120.0))
                            .hint_text("Yêu cầu AI tạo script, UI, dựng map hoặc test..."),
                    );
                    let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if (ui.button("Send").clicked() || enter) && !st.prompt_input.trim().is_empty() && !st.busy {
                        send_prompt = true;
                    }
                });
            });
        });

    if !win_open {
        with_state(|st| st.open = false);
    }
    if do_probe {
        probe_roblox_assistant(ed, ctx);
    }
    if do_start_srv {
        start_mcp_server(ctx);
    }
    if do_undo_all {
        let count = with_state(|st| std::mem::take(&mut st.ai_undo_waypoints));
        for _ in 0..count {
            ed.undo();
        }
    }
    if !approve_ids.is_empty() {
        with_state(|st| st.pending_tool_queue.extend(approve_ids));
    }
    if send_prompt {
        with_state(|st| {
            let prompt = std::mem::take(&mut st.prompt_input);
            st.messages.push(ChatMessage {
                role: "user".to_owned(),
                text: prompt.clone(),
            });
            st.conversation_history
                .push(json!({ "role": "user", "content": prompt }));
            st.agent_step_budget = 6;
            if st.provider == ProviderKind::RobloxOfficial && st.roblox_probe != RobloxProbeState::Available {
                st.provider = if !st.groq_api_key.is_empty() && st.gemini_api_key.is_empty() {
                    ProviderKind::GroqFree
                } else {
                    ProviderKind::GeminiFree
                };
            }
        });
        dispatch_llm_turn(ed, ctx);
    }
}
