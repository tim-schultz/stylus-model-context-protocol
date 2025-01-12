use alloy::{providers::RootProvider, transports::BoxTransport};
use anyhow::Result;
use eng_assistant::assistant::Assistant;
use mcp_sdk::server::Server;
use mcp_sdk::transport::ServerStdioTransport;
use mcp_sdk::types::{
    CallToolRequest, CallToolResponse, ListRequest, ResourcesListResponse, ServerCapabilities,
    Tool, ToolResponseContent, ToolsListResponse,
};
use serde_json::json;
use std::sync::Arc;
use stylus_context_provider::{
    contract_components::ContractComponents, contract_interactions::ContractInteraction,
    prompts::SMART_CONTRACT_PARSER_SYSTEM_PROMPT, StylusContract,
};
const MODEL: &str = "claude-3-5-sonnet-20241022";
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        // needs to be stderr due to stdio transport
        .with_writer(std::io::stderr)
        .init();

    let dir = std::env::var("CONTRACT_DIRECTORY")
        .expect("You must specify the directory that that contract lives in");

    // Initialize contract components
    let abi_export = std::process::Command::new("cargo")
        .arg("stylus")
        .arg("export-abi")
        .current_dir(dir.clone())
        .output()
        .expect("Failed to execute cargo stylus export-abi");

    let abi_output =
        String::from_utf8(abi_export.stdout).expect("Failed to parse command output as UTF-8");

    let contract = StylusContract::new(&dir);
    let contract_skeleton = contract.analyze().expect("Failed to analyze contract");

    let cache_path = "parsed_contract.txt";
    let parsed_sc_text = if std::path::Path::new(cache_path).exists() {
        std::fs::read_to_string(cache_path).expect("Failed to read cached contract analysis")
    } else {
        let mut assistant = Assistant::new(
            MODEL,
            false,
            None,
            None,
            Some(SMART_CONTRACT_PARSER_SYSTEM_PROMPT.to_string()),
        )
        .expect("Failed to initialize Claude");

        let prompt = format!(
            "Claude, analyze the following Rust smart contract: \n{}",
            contract_skeleton
        );

        let parse_sc_response = assistant
            .send_message(&prompt, true)
            .await
            .expect("Failed to get assistant response");

        let text = parse_sc_response
            .content
            .iter()
            .find_map(|content| match content {
                anthropic_sdk::ContentItem::Text { text } => Some(text.clone()),
                _ => None,
            })
            .expect("No text content in response");

        std::fs::write(cache_path, &text).expect("Failed to cache contract analysis");
        text
    };

    let contract_components = ContractComponents::new(&parsed_sc_text, Some(&abi_output));
    let contract_interaction = Arc::new(
        ContractInteraction::new(&abi_output, contract_components)
            .expect("Failed to create contract interaction"),
    );

    contract_interaction.build_contract_claude_tool_definitions()?;

    let server = Server::builder(ServerStdioTransport)
        .capabilities(ServerCapabilities {
            tools: Some(json!({})),
            ..Default::default()
        })
        .request_handler("tools/list", list_tools)
        .request_handler("tools/call", {
            move |req: CallToolRequest| {
                let contract_interaction = Arc::clone(&contract_interaction);
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current()
                        .block_on(async move { call_tool(req, &contract_interaction).await })
                })
            }
        })
        .request_handler("resources/list", |_req: ListRequest| {
            Ok(ResourcesListResponse {
                resources: vec![],
                next_cursor: None,
                meta: None,
            })
        })
        .build();
    let server_handle = {
        let server = server;
        tokio::spawn(async move { server.listen().await })
    };

    server_handle
        .await?
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;
    Ok(())
}

#[allow(dead_code)]
async fn call_tool(
    req: CallToolRequest,
    contract: &ContractInteraction<RootProvider<BoxTransport>>,
) -> Result<CallToolResponse> {
    let name = req.name.as_str();
    let args = req.arguments.unwrap_or_default();

    // Extract all string parameters from args into a Vec
    let mut params: Vec<String> = Vec::new();
    for (key, value) in args.iter() {
        if let Some(str_val) = value.as_str() {
            // Skip the "value" parameter as it's handled separately
            if key != "value" {
                params.push(str_val.to_string());
            }
        }
    }

    // Extract value parameter if present
    let value = args
        .get("value")
        .and_then(|v| v.as_str())
        .and_then(|v| v.parse::<u128>().ok());

    // Call the function and get result
    contract.call_function(name, Some(params), value).await?;

    Ok(CallToolResponse {
        content: vec![ToolResponseContent::Text {
            text: format!("Function {} called successfully", name),
        }],
        is_error: None,
        meta: None,
    })
}

#[derive(Debug, serde::Deserialize)]
struct ToolsJson {
    tools: Vec<Tool>,
}

fn list_tools(_req: ListRequest) -> Result<ToolsListResponse> {
    let json_str = include_str!("../../read_write_contract_tools.json");
    let tools_json: ToolsJson = serde_json::from_str(json_str)?;
    dbg!(&tools_json.tools);
    Ok(ToolsListResponse {
        tools: tools_json.tools,
        next_cursor: None,
        meta: None,
    })
}
