use farscry_core::{
    Confidence, ElementType, ScreenType, StateId, UiElement, VaspDelta, VaspOutput,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, UnixListener};

pub trait PipelineOps: Clone + Send + 'static {
    fn process(&self, image_path: &str) -> Result<VaspOutput, String>;

    fn diff(
        &self,
        before: &VaspOutput,
        after: &VaspOutput,
        before_dims: Option<(u32, u32)>,
        after_dims: Option<(u32, u32)>,
    ) -> VaspDelta;
}

#[derive(Clone)]
pub struct MockPipeline;

impl Default for MockPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl MockPipeline {
    pub fn new() -> Self {
        Self
    }
}

impl PipelineOps for MockPipeline {
    fn process(&self, _image_path: &str) -> Result<VaspOutput, String> {
        Ok(VaspOutput::new(
            StateId::from_bits(0x123456789ABCDEF0),
            ScreenType::Ui,
            Confidence::High,
            "eng",
            "Mock extraction",
            vec![UiElement {
                text: "Test Element".to_string(),
                element_type: ElementType::Label,
                cx: 100.0,
                cy: 100.0,
                w: 100.0,
                h: 30.0,
                enabled: None,
                value: None,
            }],
            vec![],
        ))
    }

    fn diff(
        &self,
        before: &VaspOutput,
        after: &VaspOutput,
        _before_dims: Option<(u32, u32)>,
        _after_dims: Option<(u32, u32)>,
    ) -> VaspDelta {
        VaspDelta {
            vasp_version: "1.0".to_string(),
            diff_from: before.state_id,
            diff_to: after.state_id,
            context_similarity: 1.0,
            context_changed: false,
            agent_context: "Mock diff".to_string(),
            entries: vec![],
            tokens_saved: Some(100),
        }
    }
}

#[derive(Clone)]
pub struct McpServer<P> {
    pipeline: Arc<Mutex<P>>,
    last_state: Arc<Mutex<Option<VaspOutput>>>,
}

impl<P: PipelineOps> McpServer<P> {
    pub fn new(pipeline: P) -> Self {
        Self {
            pipeline: Arc::new(Mutex::new(pipeline)),
            last_state: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn serve_unix_with(socket_path: &Path, pipeline: P) -> Result<(), String> {
        if socket_path.exists() {
            std::fs::remove_file(socket_path)
                .map_err(|e| format!("Failed to remove socket: {e}"))?;
        }
        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {e}"))?;
        }
        let listener =
            UnixListener::bind(socket_path).map_err(|e| format!("Failed to bind socket: {e}"))?;
        let server = McpServer::new(pipeline);
        eprintln!(
            "[farscry] MCP server listening on {}",
            socket_path.display()
        );
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let server_clone = server.clone();
                    tokio::spawn(async move {
                        if let Err(e) = server_clone.handle_unix_session(stream).await {
                            eprintln!("[farscry] session error: {e}");
                        }
                    });
                }
                Err(e) => eprintln!("[farscry] accept error: {e}"),
            }
        }
    }

    pub async fn serve_tcp_with(port: u16, pipeline: P) -> Result<(), String> {
        let addr = format!("127.0.0.1:{port}");
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| format!("Failed to bind TCP: {e}"))?;
        let server = McpServer::new(pipeline);
        eprintln!("[farscry] MCP server listening on {addr}");
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let server_clone = server.clone();
                    tokio::spawn(async move {
                        if let Err(e) = server_clone.handle_tcp_session(stream).await {
                            eprintln!("[farscry] session error: {e}");
                        }
                    });
                }
                Err(e) => eprintln!("[farscry] accept error: {e}"),
            }
        }
    }
}

impl McpServer<MockPipeline> {
    pub async fn serve_unix(socket_path: &Path) -> Result<(), String> {
        McpServer::<MockPipeline>::serve_unix_with(socket_path, MockPipeline::new()).await
    }

    pub async fn serve_tcp(port: u16) -> Result<(), String> {
        McpServer::<MockPipeline>::serve_tcp_with(port, MockPipeline::new()).await
    }
}

impl<P: PipelineOps> McpServer<P> {
    async fn handle_unix_session(&self, stream: tokio::net::UnixStream) -> Result<(), String> {
        let (reader, writer) = tokio::io::split(stream);
        self.run_session(BufReader::new(reader), writer).await
    }

    async fn handle_tcp_session(&self, stream: tokio::net::TcpStream) -> Result<(), String> {
        let (reader, writer) = tokio::io::split(stream);
        self.run_session(BufReader::new(reader), writer).await
    }

    async fn run_session<R, W>(&self, mut reader: BufReader<R>, mut writer: W) -> Result<(), String>
    where
        R: tokio::io::AsyncRead + Unpin,
        W: tokio::io::AsyncWrite + Unpin,
    {
        let mut line = String::new();

        loop {
            line.clear();
            let n = reader
                .read_line(&mut line)
                .await
                .map_err(|e| format!("read error: {e}"))?;
            if n == 0 {
                break;
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let msg: Value = match serde_json::from_str(trimmed) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("[farscry] JSON parse error: {e}");

                    let err_resp = error_response(Value::Null, -32700, "Parse error");
                    send_line(&mut writer, &err_resp).await?;
                    continue;
                }
            };

            if let Some(response) = self.dispatch(&msg).await {
                send_line(&mut writer, &response).await?;
            }
        }

        Ok(())
    }

    async fn dispatch(&self, msg: &Value) -> Option<Value> {
        let method = msg.get("method")?.as_str()?;

        let id_opt = msg.get("id");
        let is_notification = id_opt.is_none() || id_opt.map(Value::is_null).unwrap_or(false);
        if is_notification {
            return None;
        }
        let id = id_opt.cloned().unwrap_or(Value::Null);

        let params = msg.get("params").cloned().unwrap_or(Value::Null);

        let result: Result<Value, Value> = match method {
            "initialize" => self.mcp_initialize(&params),
            "tools/list" => self.mcp_tools_list(),
            "tools/call" => self.mcp_tools_call(&params).await,

            "farscry_extract" | "farscry_diff" => {
                let req = JsonRpcRequest {
                    _jsonrpc: "2.0".to_string(),
                    method: method.to_string(),
                    params,
                    id: id.clone(),
                };
                let resp = self.handle_request(&req).await;
                match (resp.result, resp.error) {
                    (Some(r), _) => Ok(r),
                    (_, Some(e)) => Err(serde_json::json!({"code": e.code, "message": e.message})),
                    _ => Ok(Value::Null),
                }
            }
            other => Err(serde_json::json!({
                "code": -32601,
                "message": format!("Method not found: {other}")
            })),
        };

        Some(match result {
            Ok(r) => serde_json::json!({"jsonrpc": "2.0", "result": r, "id": id}),
            Err(e) => serde_json::json!({"jsonrpc": "2.0", "error": e, "id": id}),
        })
    }

    fn mcp_initialize(&self, _params: &Value) -> Result<Value, Value> {
        Ok(serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "farscry",
                "version": "0.1.0"
            }
        }))
    }

    fn mcp_tools_list(&self) -> Result<Value, Value> {
        Ok(serde_json::json!({
            "tools": [
                {
                    "name": "farscry_extract",
                    "description": "Converts any screenshot into VASP structured context for automation workflows. Returns typed, coordinate-rich UI elements with positions and affordances (click/type targets).",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "image_path": {
                                "type": "string",
                                "description": "Absolute path to the image file (PNG, JPG, WebP, GIF)"
                            },
                            "lang": {
                                "type": "string",
                                "description": "OCR language code, e.g. 'eng', 'por', 'chi_sim'",
                                "default": "eng"
                            },
                            "affordances": {
                                "type": "boolean",
                                "description": "Include affordances (click/type targets) in output",
                                "default": true
                            }
                        },
                        "required": ["image_path"]
                    }
                },
                {
                    "name": "farscry_diff",
                    "description": "Returns semantic delta between two screenshots - appeared, changed, removed elements. Saves tokens vs re-sending full images to vision APIs.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "before": {
                                "type": "string",
                                "description": "Absolute path to the before-action image"
                            },
                            "after": {
                                "type": "string",
                                "description": "Absolute path to the after-action image"
                            }
                        },
                        "required": ["before", "after"]
                    }
                }
            ]
        }))
    }

    async fn mcp_tools_call(&self, params: &Value) -> Result<Value, Value> {
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| mcp_error(-32602, "Missing tool name in params"))?;

        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or(Value::Object(serde_json::Map::new()));

        match name {
            "farscry_extract" => {
                let image_path = arguments
                    .get("image_path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| mcp_error(-32602, "Missing required argument: image_path"))?
                    .to_string();

                let (img_w, img_h) = image::image_dimensions(&image_path).unwrap_or((1920, 1080));
                let image_path_for_fmt = image_path.clone();

                let pipeline = self.pipeline.clone();
                let result = tokio::task::spawn_blocking(move || {
                    pipeline.lock().unwrap().process(&image_path)
                })
                .await
                .map_err(|e| mcp_error(-32000, &format!("Spawn error: {e}")))?;

                match result {
                    Ok(output) => {
                        let auto_diff = self.compute_auto_diff(&output).await;
                        *self.last_state.lock().unwrap() = Some(output.clone());

                        let vasp_text = farscry_formatter::VaspFormatter::format_vasp(
                            &output,
                            &image_path_for_fmt,
                            img_w,
                            img_h,
                        );
                        let text = match auto_diff {
                            Some(ref d) => format!(
                                "{vasp_text}\n=== diff from previous state ===\n{}",
                                farscry_formatter::VaspFormatter::format_diff(d)
                            ),
                            None => vasp_text,
                        };
                        Ok(tool_result_text(&text))
                    }
                    Err(e) => Ok(tool_result_error(&format!("farscry_extract failed: {e}"))),
                }
            }

            "farscry_diff" => {
                let before_path = arguments
                    .get("before")
                    .and_then(Value::as_str)
                    .ok_or_else(|| mcp_error(-32602, "Missing required argument: before"))?
                    .to_string();
                let after_path = arguments
                    .get("after")
                    .and_then(Value::as_str)
                    .ok_or_else(|| mcp_error(-32602, "Missing required argument: after"))?
                    .to_string();

                let before_dims = image::image_dimensions(&before_path).ok();
                let after_dims = image::image_dimensions(&after_path).ok();

                let pipeline1 = self.pipeline.clone();
                let pipeline2 = self.pipeline.clone();

                let (before_result, after_result) = tokio::join!(
                    tokio::task::spawn_blocking(move || pipeline1
                        .lock()
                        .unwrap()
                        .process(&before_path)),
                    tokio::task::spawn_blocking(move || pipeline2
                        .lock()
                        .unwrap()
                        .process(&after_path))
                );

                let before = before_result
                    .map_err(|e| mcp_error(-32000, &format!("Spawn error: {e}")))?
                    .map_err(|e| mcp_error(-32000, &format!("before failed: {e}")))?;
                let after = after_result
                    .map_err(|e| mcp_error(-32000, &format!("Spawn error: {e}")))?
                    .map_err(|e| mcp_error(-32000, &format!("after failed: {e}")))?;

                let pipeline3 = self.pipeline.clone();
                let delta = tokio::task::spawn_blocking(move || {
                    pipeline3
                        .lock()
                        .unwrap()
                        .diff(&before, &after, before_dims, after_dims)
                })
                .await
                .map_err(|e| mcp_error(-32000, &format!("Spawn error: {e}")))?;

                Ok(tool_result_text(
                    &farscry_formatter::VaspFormatter::format_diff(&delta),
                ))
            }

            other => Err(mcp_error(-32602, &format!("Unknown tool: {other}"))),
        }
    }

    async fn compute_auto_diff(&self, current: &VaspOutput) -> Option<VaspDelta> {
        let last = self.last_state.lock().unwrap().clone()?;
        let pipeline = self.pipeline.clone();
        let current_clone = current.clone();
        tokio::task::spawn_blocking(move || {
            pipeline
                .lock()
                .unwrap()
                .diff(&last, &current_clone, None, None)
        })
        .await
        .ok()
    }

    pub async fn handle_request(&self, request: &JsonRpcRequest) -> JsonRpcResponse {
        let result = match request.method.as_str() {
            "farscry_extract" => self.handle_extract(request).await,
            "farscry_diff" => self.handle_diff(request).await,
            other => Err(format!("Unknown method: {other}")),
        };

        match result {
            Ok(result) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(Value::String(result)),
                error: None,
                id: request.id.clone(),
            },
            Err(error) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32000,
                    message: error,
                }),
                id: request.id.clone(),
            },
        }
    }

    async fn handle_extract(&self, request: &JsonRpcRequest) -> Result<String, String> {
        let params: ExtractParams = serde_json::from_value(request.params.clone())
            .map_err(|e| format!("Invalid params: {e}"))?;

        let image_path = params.image_path;
        let (img_w, img_h) = image::image_dimensions(&image_path).unwrap_or((1920, 1080));
        let image_path_for_fmt = image_path.clone();

        let pipeline = self.pipeline.clone();
        let result =
            tokio::task::spawn_blocking(move || pipeline.lock().unwrap().process(&image_path))
                .await
                .map_err(|e| format!("Task error: {e}"))??;

        let auto_diff = self.compute_auto_diff(&result).await;
        *self.last_state.lock().unwrap() = Some(result.clone());

        let vasp_text = farscry_formatter::VaspFormatter::format_vasp(
            &result,
            &image_path_for_fmt,
            img_w,
            img_h,
        );
        let text = match auto_diff {
            Some(ref d) => format!(
                "{vasp_text}\n=== diff from previous state ===\n{}",
                farscry_formatter::VaspFormatter::format_diff(d)
            ),
            None => vasp_text,
        };
        Ok(text)
    }

    async fn handle_diff(&self, request: &JsonRpcRequest) -> Result<String, String> {
        let params: DiffParams = serde_json::from_value(request.params.clone())
            .map_err(|e| format!("Invalid params: {e}"))?;

        let before_path = params.before;
        let after_path = params.after;

        let before_dims = image::image_dimensions(&before_path).ok();
        let after_dims = image::image_dimensions(&after_path).ok();

        let pipeline1 = self.pipeline.clone();
        let pipeline2 = self.pipeline.clone();

        let (before_result, after_result) = tokio::join!(
            tokio::task::spawn_blocking(move || pipeline1.lock().unwrap().process(&before_path)),
            tokio::task::spawn_blocking(move || pipeline2.lock().unwrap().process(&after_path))
        );

        let before = before_result.map_err(|e| format!("Task error: {e}"))??;
        let after = after_result.map_err(|e| format!("Task error: {e}"))??;

        let pipeline3 = self.pipeline.clone();
        let delta = tokio::task::spawn_blocking(move || {
            pipeline3
                .lock()
                .unwrap()
                .diff(&before, &after, before_dims, after_dims)
        })
        .await
        .map_err(|e| format!("Task error: {e}"))?;

        Ok(farscry_formatter::VaspFormatter::format_diff(&delta))
    }
}

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    #[serde(rename = "jsonrpc")]
    pub _jsonrpc: String,
    pub method: String,
    pub params: Value,
    pub id: Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub result: Option<Value>,
    pub error: Option<JsonRpcError>,
    pub id: Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Deserialize)]
struct ExtractParams {
    image_path: String,
    #[serde(default = "default_lang", rename = "lang")]
    _lang: String,
    #[serde(default = "default_affordances", rename = "affordances")]
    _affordances: bool,
}

fn default_lang() -> String {
    "eng".to_string()
}
fn default_affordances() -> bool {
    true
}

#[derive(Debug, Deserialize)]
struct DiffParams {
    before: String,
    after: String,
}

async fn send_line<W: tokio::io::AsyncWrite + Unpin>(
    writer: &mut W,
    value: &Value,
) -> Result<(), String> {
    let mut line = serde_json::to_string(value).map_err(|e| format!("JSON serialize: {e}"))?;
    line.push('\n');
    writer
        .write_all(line.as_bytes())
        .await
        .map_err(|e| format!("write error: {e}"))
}

fn error_response(id: Value, code: i32, message: &str) -> Value {
    serde_json::json!({"jsonrpc":"2.0","error":{"code":code,"message":message},"id":id})
}

fn mcp_error(code: i32, message: &str) -> Value {
    serde_json::json!({"code": code, "message": message})
}

fn tool_result_text(text: &str) -> Value {
    serde_json::json!({
        "content": [{"type": "text", "text": text}],
        "isError": false
    })
}

fn tool_result_error(message: &str) -> Value {
    serde_json::json!({
        "content": [{"type": "text", "text": message}],
        "isError": true
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_pipeline_process() {
        let pipeline = MockPipeline::new();
        let result = pipeline.process("test.png");
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.ui_tree.len(), 1);
        assert_eq!(output.ui_tree[0].text, "Test Element");
    }

    #[test]
    fn test_mock_pipeline_diff() {
        let pipeline = MockPipeline::new();
        let before = pipeline.process("before.png").unwrap();
        let after = pipeline.process("after.png").unwrap();
        let delta = pipeline.diff(&before, &after, None, None);
        assert_eq!(delta.context_similarity, 1.0);
        assert!(!delta.context_changed);
    }

    #[tokio::test]
    async fn test_server_starts_unix() {
        let temp_dir = tempfile::tempdir().unwrap();
        let socket_path = temp_dir.path().join("test.sock");

        let socket_path_clone = socket_path.clone();
        tokio::spawn(async move { McpServer::serve_unix(&socket_path_clone).await });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        assert!(socket_path.exists());
    }

    #[test]
    fn test_json_rpc_request_parse() {
        let json = r#"{"jsonrpc":"2.0","method":"farscry_extract","params":{"image_path":"test.png"},"id":1}"#;
        let request: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.method, "farscry_extract");
        assert_eq!(request._jsonrpc, "2.0");
    }

    #[test]
    fn test_extract_params_parse() {
        let json = r#"{"image_path":"test.png","lang":"eng","affordances":true}"#;
        let params: ExtractParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.image_path, "test.png");
        assert_eq!(params._lang, "eng");
        assert!(params._affordances);
    }

    #[test]
    fn test_diff_params_parse() {
        let json = r#"{"before":"before.png","after":"after.png"}"#;
        let params: DiffParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.before, "before.png");
        assert_eq!(params.after, "after.png");
    }

    #[tokio::test]
    async fn test_handle_extract() {
        let server = McpServer::new(MockPipeline::new());
        let request = JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            method: "farscry_extract".to_string(),
            params: serde_json::json!({"image_path": "test.png"}),
            id: serde_json::json!(1),
        };
        let response = server.handle_request(&request).await;
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_handle_diff() {
        let server = McpServer::new(MockPipeline::new());
        let request = JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            method: "farscry_diff".to_string(),
            params: serde_json::json!({"before": "before.png", "after": "after.png"}),
            id: serde_json::json!(1),
        };
        let response = server.handle_request(&request).await;
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_concurrent_requests() {
        let server = McpServer::new(MockPipeline::new());
        let server_clone = server.clone();

        let req1 = JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            method: "farscry_extract".to_string(),
            params: serde_json::json!({"image_path": "test1.png"}),
            id: serde_json::json!(1),
        };
        let req2 = JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            method: "farscry_extract".to_string(),
            params: serde_json::json!({"image_path": "test2.png"}),
            id: serde_json::json!(2),
        };

        let (r1, r2) = tokio::join!(
            server.handle_request(&req1),
            server_clone.handle_request(&req2)
        );
        assert!(r1.result.is_some());
        assert!(r2.result.is_some());
    }

    #[tokio::test]
    async fn test_mcp_initialize() {
        let server = McpServer::new(MockPipeline::new());
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test-client", "version": "1.0"}
            },
            "id": 1
        });

        let response = server.dispatch(&msg).await.unwrap();
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);
        let result = &response["result"];
        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert_eq!(result["serverInfo"]["name"], "farscry");
        assert!(result["capabilities"]["tools"].is_object());
    }

    #[tokio::test]
    async fn test_mcp_notifications_initialized_no_response() {
        let server = McpServer::new(MockPipeline::new());
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"

        });

        let response = server.dispatch(&msg).await;
        assert!(
            response.is_none(),
            "notifications must not produce a response"
        );
    }

    #[tokio::test]
    async fn test_mcp_tools_list() {
        let server = McpServer::new(MockPipeline::new());
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "tools/list",
            "params": {},
            "id": 2
        });

        let response = server.dispatch(&msg).await.unwrap();
        assert_eq!(response["jsonrpc"], "2.0");
        let tools = &response["result"]["tools"];
        assert!(tools.is_array());
        let tools_arr = tools.as_array().unwrap();
        assert_eq!(tools_arr.len(), 2);

        let tool_names: Vec<&str> = tools_arr
            .iter()
            .filter_map(|t| t["name"].as_str())
            .collect();
        assert!(tool_names.contains(&"farscry_extract"));
        assert!(tool_names.contains(&"farscry_diff"));

        for tool in tools_arr {
            assert!(
                tool["inputSchema"].is_object(),
                "tool must have inputSchema"
            );
            assert!(
                tool["description"].is_string(),
                "tool must have description"
            );
        }
    }

    #[tokio::test]
    async fn test_mcp_tools_call_extract() {
        let server = McpServer::new(MockPipeline::new());
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "farscry_extract",
                "arguments": {"image_path": "test.png"}
            },
            "id": 3
        });

        let response = server.dispatch(&msg).await.unwrap();
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 3);
        let result = &response["result"];
        assert!(result["content"].is_array());
        assert_eq!(result["content"][0]["type"], "text");
        assert!(result["content"][0]["text"].is_string());
        assert_eq!(result["isError"], false);
    }

    #[tokio::test]
    async fn test_mcp_tools_call_diff() {
        let server = McpServer::new(MockPipeline::new());
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "farscry_diff",
                "arguments": {"before": "before.png", "after": "after.png"}
            },
            "id": 4
        });

        let response = server.dispatch(&msg).await.unwrap();
        assert_eq!(response["jsonrpc"], "2.0");
        let result = &response["result"];
        assert_eq!(result["isError"], false);
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("farscry diff"));
    }

    #[tokio::test]
    async fn test_mcp_tools_call_unknown_tool() {
        let server = McpServer::new(MockPipeline::new());
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "nonexistent_tool",
                "arguments": {}
            },
            "id": 5
        });

        let response = server.dispatch(&msg).await.unwrap();

        assert!(response["error"].is_object());
    }

    #[tokio::test]
    async fn test_mcp_unknown_method() {
        let server = McpServer::new(MockPipeline::new());
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "unknown/method",
            "params": {},
            "id": 6
        });

        let response = server.dispatch(&msg).await.unwrap();
        assert!(response["error"].is_object());
        assert_eq!(response["error"]["code"], -32601);
    }

    #[tokio::test]
    async fn test_full_mcp_handshake_sequence() {
        let server = McpServer::new(MockPipeline::new());

        let init = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name":"test","version":"1"}},
            "id": 1
        });
        let r = server.dispatch(&init).await.unwrap();
        assert_eq!(r["result"]["protocolVersion"], "2024-11-05");

        let notif = serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"});
        assert!(server.dispatch(&notif).await.is_none());

        let list = serde_json::json!({"jsonrpc":"2.0","method":"tools/list","params":{},"id":2});
        let r = server.dispatch(&list).await.unwrap();
        assert!(r["result"]["tools"].is_array());

        let call = serde_json::json!({
            "jsonrpc":"2.0","method":"tools/call",
            "params":{"name":"farscry_extract","arguments":{"image_path":"test.png"}},
            "id":3
        });
        let r = server.dispatch(&call).await.unwrap();
        assert_eq!(r["result"]["isError"], false);
    }
}
