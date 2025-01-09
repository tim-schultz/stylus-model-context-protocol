use alloy::{providers::RootProvider, transports::BoxTransport};
use anyhow::{Context, Result};
use contract_components::ContractComponents;
use contract_interactions::ContractInteraction;
use eng_assistant::assistant::Assistant;
use env_logger::Env;
use log::info;

use std::env;
use stylus_context_provider::StylusContract;

mod contract_components;
mod contract_interactions;
mod prompts;

use prompts::SMART_CONTRACT_PARSER_SYSTEM_PROMPT;

pub const MODEL: &str = "claude-3-5-sonnet-20241022";
pub const TASK_COMPLETE: &str = "TASK_COMPLETE";

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // CLI Param
    let args: Vec<String> = env::args().collect();
    let dir = args
        .get(1)
        .map(|s| s.as_str())
        .expect("You must pass a directory");

    // Execute cargo stylus export-abi command
    let abi_export = std::process::Command::new("cargo")
        .arg("stylus")
        .arg("export-abi")
        .current_dir(dir)
        .output()
        .context("Failed to execute cargo stylus export-abi")?;

    let abi_output =
        String::from_utf8(abi_export.stdout).context("Failed to parse command output as UTF-8")?;

    // Instantiate Stylus Contract - Used to parse rust smart contract code and get as much information about the contract as possible
    let contract = StylusContract::new(dir);
    // Use ruskel to parse the rust API from the contract and get a skeleton of the contract and comments
    let contract_skeleton = contract.analyze()?;

    // Instantiate Claude Assistant Client
    let mut stylus_smart_contract_parser_assistant = Assistant::new(
        MODEL,
        false,
        None,
        None,
        Some(SMART_CONTRACT_PARSER_SYSTEM_PROMPT.to_string()),
    )
    .context("Failed to initialize Claude")?;
    info!("Claude instance initialized with model: {}", MODEL);

    // Ask claude to analyze the rust smart contract and provide a detailed report
    let prompt = format!(
        "Claude, analyze the following Rust smart contract: \n{}",
        contract_skeleton
    );
    let response = stylus_smart_contract_parser_assistant
        .send_message(&prompt, true)
        .await?;
    info!("Response: {:?}", response);
    let text = response
        .content
        .iter()
        .find_map(|content| match content {
            anthropic_sdk::ContentItem::Text { text } => Some(text.clone()),
            _ => None,
        })
        .context("No text content in response")?;

    // Use structured output from Claude Assistant to parse different aspects of the contract
    let contract_components = ContractComponents::new(&text, Some(&abi_output));

    // Instantiate Smart Contract Interaction
    let contract_interaction =
        ContractInteraction::<RootProvider<BoxTransport>>::new(&abi_output, contract_components)?;

    dbg!(contract_interaction.build_contract_claude_tool_definitions()?);

    Ok(())
}
