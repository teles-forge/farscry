use anyhow::Result;
use farscry_core::Pipeline;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
struct FarscryPipelineAdapter {
    pipeline: Arc<Pipeline>,
}

impl farscry_mcp::PipelineOps for FarscryPipelineAdapter {
    fn process(&self, image_path: &str) -> Result<farscry_core::VaspOutput, String> {
        let img = image::open(image_path).map_err(|e| format!("cannot open image: {e}"))?;
        self.pipeline.process(img).map_err(|e| e.to_string())
    }

    fn diff(
        &self,
        before: &farscry_core::VaspOutput,
        after: &farscry_core::VaspOutput,
        before_dims: Option<(u32, u32)>,
        after_dims: Option<(u32, u32)>,
    ) -> farscry_core::VaspDelta {
        use farscry_core::DiffEngine;
        farscry_diff::DiffEngineImpl.diff(before, after, before_dims, after_dims)
    }
}

pub async fn serve_mcp(mcp: bool, port: Option<u16>) -> Result<()> {
    if !mcp {
        anyhow::bail!("Only MCP mode is currently supported");
    }

    let pipeline = crate::pipeline::get_or_build_pipeline()
        .map_err(|e| anyhow::anyhow!("Pipeline init failed: {e}"))?;
    let adapter = FarscryPipelineAdapter { pipeline };

    if let Some(port) = port {
        farscry_mcp::McpServer::serve_tcp_with(port, adapter)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
    } else {
        #[cfg(unix)]
        {
            let socket_path = dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".farscry")
                .join("mcp.sock");
            farscry_mcp::McpServer::serve_unix_with(&socket_path, adapter)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
        }
        #[cfg(not(unix))]
        {
            anyhow::bail!(
                "Unix Domain Sockets are not supported on Windows. Use --port to specify a TCP port."
            );
        }
    }

    Ok(())
}
