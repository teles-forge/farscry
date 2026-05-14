use serde_json::Value;
use tokio::io::AsyncWriteExt;

pub(crate) async fn send_line<W: tokio::io::AsyncWrite + Unpin>(
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

pub(crate) fn error_response(id: Value, code: i32, message: &str) -> Value {
    serde_json::json!({"jsonrpc":"2.0","error":{"code":code,"message":message},"id":id})
}

pub(crate) fn mcp_error(code: i32, message: &str) -> Value {
    serde_json::json!({"code": code, "message": message})
}

pub(crate) fn tool_result_text(text: &str) -> Value {
    serde_json::json!({
        "content": [{"type": "text", "text": text}],
        "isError": false
    })
}

pub(crate) fn tool_result_error(message: &str) -> Value {
    serde_json::json!({
        "content": [{"type": "text", "text": message}],
        "isError": true
    })
}
