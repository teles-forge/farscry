Story 8 - farscry-mcp

**Status:** Ready
**Blocked by:** Story 1 (farscry-core)
**Estimated hours:** 8h

What to build

MCP server exposing `farscry_extract` and `farscry_diff` over Unix Domain Socket (default) or TCP 127.0.0.1 (--port flag). Daemon mode: keeps models warm, diffs automatically.

Crate: `crates/farscry-mcp/`

**Critical concurrency rules (spec-mandated):**
1. `Arc<Mutex<Pipeline>>` - ONNX Runtime sessions are not Sync; serialize inference
2. NEVER hold Mutex across `.await`
3. All inference via `tokio::task::spawn_blocking`

```rust
async fn handle_request(stream: UnixStream, pipeline: Arc<Mutex<Pipeline>>) {
    let image = parse_image_from_mcp(&stream).await;  // async read - no lock


    let result = tokio::task::spawn_blocking(move || {
        let pipeline = pipeline.lock().unwrap();
        pipeline.process(image)
    }).await.unwrap();

    write_vasp_to_stream(&stream, result).await;  // async write - no lock
}
```

**`src/lib.rs`** - MCP server:

```rust
pub struct McpServer {
    pipeline: Arc<Mutex<Pipeline>>,
    last_state: Arc<Mutex<Option<VaspOutput>>>,  // for auto-diff in daemon mode
}

impl McpServer {
    pub async fn serve_unix(socket_path: &Path, pipeline: Pipeline) -> Result<(), FarscryError>;
    pub async fn serve_tcp(port: u16, pipeline: Pipeline) -> Result<(), FarscryError>;

}
```

**Two MCP tools:**

`farscry_extract` - extract VASP from image:
- Input: `{ "image_path": string, "lang": string = "eng", "affordances": bool = true }`
- Process: pipeline.process(image)
- Auto-diff: if `last_state` exists and context_similarity >= 0.20, compute and include delta
- Store result as new `last_state`
- Output: VASP text (default) or JSON

`farscry_diff` - explicit diff between two images:
- Input: `{ "before": string, "after": string }`
- Process: pipeline.process(before) + pipeline.process(after) -> diff_engine.diff()
- Output: VaspDelta in VASP format

**MCP protocol (JSON-RPC 2.0):**
Follow MCP spec at https://modelcontextprotocol.io/specification. Use `mcp-server` crate or implement JSON-RPC 2.0 manually.

**Default socket path:** `~/.farscry/mcp.sock`

**`Cargo.toml`:**
```toml
[dependencies]
farscry-core = { path = "../farscry-core" }
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

Acceptance criteria

- [ ] `cargo test -p farscry-mcp` passes
- [ ] `farscry serve --mcp` starts and accepts connections at `~/.farscry/mcp.sock`
- [ ] `farscry serve --mcp --port 3333` binds TCP on 127.0.0.1:3333 only
- [ ] Test: TCP with `--port 0` -> binds 127.0.0.1, never 0.0.0.0
- [ ] `farscry_extract` tool responds with VASP output
- [ ] `farscry_diff` tool responds with delta output
- [ ] Two consecutive `farscry_extract` calls: second response includes auto-diff delta
- [ ] Mutex is NOT held across any `.await` - verify by code review
- [ ] All inference goes through `tokio::task::spawn_blocking`
- [ ] MCP client `.claude/mcp.json` config snippet connects successfully
- [ ] MCP client config snippet connects successfully
- [ ] publish = false

Dependencies

Story 1 (farscry-core). (Stories 2-7 are wired in Story 9 when the binary assembles the Pipeline.)
