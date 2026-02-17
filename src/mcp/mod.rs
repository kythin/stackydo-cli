pub(crate) mod prompts;
pub(crate) mod resources;
pub(crate) mod tools;

use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::router::{prompt::PromptRouter, tool::ToolRouter},
    model::*,
    prompt_handler,
    service::RequestContext,
    tool_handler,
};

#[derive(Debug, Clone)]
pub struct StackstodoMcp {
    tool_router: ToolRouter<Self>,
    prompt_router: PromptRouter<Self>,
}

impl StackstodoMcp {
    pub fn new() -> Self {
        Self {
            tool_router: tools::create_tool_router(),
            prompt_router: prompts::create_prompt_router(),
        }
    }
}

impl Default for StackstodoMcp {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_handler]
#[prompt_handler]
impl ServerHandler for StackstodoMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Stackstodo: a personal task manager. Use the stackstodo://guide resource for full documentation."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .build(),
            server_info: Implementation {
                name: "stackstodo-mcp".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                title: None,
                description: None,
                icons: None,
                website_url: None,
            },
            ..Default::default()
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult {
            meta: None,
            resources: vec![resources::guide_resource()],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        if request.uri == "stackstodo://guide" {
            Ok(ReadResourceResult {
                contents: vec![ResourceContents::TextResourceContents {
                    uri: "stackstodo://guide".into(),
                    mime_type: Some("text/markdown".into()),
                    text: resources::GUIDE_CONTENT.into(),
                    meta: None,
                }],
            })
        } else {
            Err(McpError::new(
                ErrorCode::INVALID_PARAMS,
                format!("Unknown resource: {}", request.uri),
                None,
            ))
        }
    }
}
