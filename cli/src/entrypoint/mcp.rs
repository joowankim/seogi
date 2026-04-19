use anyhow::Result;
use rmcp::ServiceExt;
use rmcp::handler::server::ServerHandler;
use rmcp::model::{Implementation, ServerCapabilities, ServerInfo};

#[derive(Clone)]
struct SeogiMcpServer;

impl ServerHandler for SeogiMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::default())
            .with_server_info(Implementation::new("seogi", env!("CARGO_PKG_VERSION")))
    }
}

/// MCP 서버를 stdio transport로 구동한다.
///
/// # Errors
///
/// tokio 런타임 초기화 실패, MCP 핸드셰이크 실패 시 `anyhow::Error`.
pub fn run() -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let server = SeogiMcpServer;
            let service = server.serve(rmcp::transport::stdio()).await?;
            service.waiting().await?;
            Ok(())
        })
}
