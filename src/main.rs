use anyhow::{Context, Result};
use eng_assistant::assistant::Assistant;
use env_logger::Env;
use log::info;
use mcp_client_rs::{ClientError, ProtocolManager};
use std::env;
use stylus_context_provider::StylusContract;

pub const MODEL: &str = "claude-3-5-sonnet-20241022";
pub const TASK_COMPLETE: &str = "TASK_COMPLETE";

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let args: Vec<String> = env::args().collect();
    let dir = args
        .get(1)
        .map(|s| s.as_str())
        .expect("You must pass a directory");

    // let mut protocol_manager = ProtocolManager::new();

    // protocol_manager
    //     .add_protocol(
    //         "sequential-thinking",
    //         vec!["-y", "@modelcontextprotocol/server-sequential-thinking"],
    //         None,
    //     )
    //     .await?;

    // let client_ids = &["sequential-thinking".to_string()];

    // let planning_prompt_tools = protocol_manager.get_tools_for_clients(Some(client_ids));

    let planning_system_prompt = r#"
    You are Claude, an AI assistant powered by Anthropic's Claude-3.5-Sonnet model, specializing in software architecture. Your capabilities include:
    - You are an expert coding assistant. 
    - You specialize in Solidity and Rust programming languages.
    - You are terse, efficient, and without emotion. You never apologize. When asked to do something you do it without preamble. 

    You will be given an outline of an arbitrum stylus smart contract which is written in Rust. It will have the following format:
    <rust_smart_contract>
        pub mod stylus_hello_world ....
    </rust_smart_contract>

    You are tasked with identifying the following implementations: Events, Errors, Storage Variables, read functions, and write functions. Below you will find examples of how each implementation may be presented:

    Event:
    <example>
        /// Event with signature `Bid(address,uint256,string)` and selector `0xf22665033e06efbfec151f6a7c6c2d108d74c528a69e4f1f98e5101219c5f2fc`.
        /// ```solidity
        /// event Bid(address indexed sender, uint256 amount, string prompt);
        /// ```
    </example>

    Error:
    <example>
        /// Custom error with signature `AlreadyStarted()` and selector `0x1fbde445`.
        /// ```solidity
        /// error AlreadyStarted();
        /// ```
    </example>

    Storage Variable:
    <example>
        pub active_prompt: stylus_sdk::storage::StorageString,
    </example>

    Read Function:
    <example>
        /// Returns the current active prompt in the auction
        /// @return The active prompt string
        pub fn active_prompt(&self) -> Result<String, EnglishAuctionError> {}
    </example>

    Write Function:
    <example>
        /// Starts the auction
        /// @dev Can only be called by the seller
        pub fn start(&mut self) -> Result<(), EnglishAuctionError> {}
    </example>

    For each occurrence of Events, Errors, Storage Variables, read functions, and write functions you will output the corresponding block in the format listed below:

    Desired Output:
    <event>
        event Bid(address indexed sender, uint256 amount, string prompt);
    </event>

    <error>
        error AlreadyStarted();
    </error>

    <storage_variable>
        pub active_prompt: stylus_sdk::storage::StorageString,
    </storage_variable>

    <read_function>
        pub fn active_prompt(&self) -> Result<String, EnglishAuctionError> {}
    </read_function>

    <write_function>
        pub fn start(&mut self) -> Result<(), EnglishAuctionError> {}
    </write_function>

    Important: Output the implementations of every Event, Error, Storage Variable, read function, and write function in the format outlined above.
    "#;

    let mut assistant = Assistant::new(
        MODEL,
        false,
        None,
        None,
        Some(planning_system_prompt.to_string()),
    )
    .context("Failed to initialize Claude")?;
    info!("Claude instance initialized with model: {}", MODEL);

    let contract = StylusContract::new(dir);
    let contract_skeletion = contract.analyze()?;
    let prompt = format!(
        "Claude, analyze the following Rust smart contract: \n{}",
        contract_skeletion
    );

    let response = assistant.send_message(&prompt, true).await?;

    dbg!(response);

    Ok(())
}
