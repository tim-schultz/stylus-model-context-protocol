use std::str::FromStr;

use alloy::{
    contract::{ContractInstance, Interface},
    dyn_abi::{DynSolValue, JsonAbiExt},
    json_abi::JsonAbi,
    primitives::{Address, Bytes},
    providers::{Provider, RootProvider},
    rpc::{
        client::RpcClient,
        types::{TransactionInput, TransactionRequest},
    },
    transports::{http, BoxTransport},
};
use anyhow::{anyhow, Result};
use dotenv::dotenv;
use regex::Regex;
use serde_json::{json, Value};
use url::Url;

use crate::contract_components::ContractComponents;

pub struct ContractInteraction<P: Provider> {
    #[allow(dead_code)]
    contract_instance: ContractInstance<Interface, P>,
    #[allow(dead_code)]
    contract_components: ContractComponents,
}

impl<P: Provider> ContractInteraction<P> {
    /// Creates a contract instance from a contract address and JSON ABI.
    pub fn build_contract(
        address: Address,
        abi: Value,
        provider: P,
    ) -> Result<ContractInstance<Interface, P>> {
        // Convert the Value array into Vec<&str>
        let abi_strings = abi
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("ABI is not an array"))?
            .iter()
            .map(|v| v.as_str().unwrap_or_default())
            .collect::<Vec<&str>>();

        let abi = match JsonAbi::parse(abi_strings) {
            Ok(abi) => abi,
            Err(e) => return Err(anyhow!("Failed to parse ABI: {}", e)),
        };

        let interface = Interface::new(abi);
        let instance = ContractInstance::new(address, provider, interface);
        Ok(instance)
    }

    pub fn build_contract_claude_tool_definitions(&self) -> Result<Value> {
        use std::fs::write;
        let mut tools = Vec::<Value>::new();

        let json_abi = self.contract_instance.abi();

        // Process all functions from ABI
        for (name, functions) in json_abi.functions.iter() {
            let function = functions.first().expect("Function should exist");
            let mut properties = serde_json::Map::new();

            // Add input parameters
            for param in function.inputs.iter() {
                properties.insert(
                    if param.name.is_empty() {
                        "input".to_string()
                    } else {
                        param.name.clone()
                    },
                    json!({
                        "type": "string",
                        "description": format!("Parameter of type {}", param.ty)
                    }),
                );
            }

            // Add value parameter for payable functions
            if matches!(
                function.state_mutability,
                alloy::json_abi::StateMutability::Payable
            ) {
                properties.insert(
                    "value".to_string(),
                    json!({
                        "type": "string",
                        "description": "Amount of native currency to send with transaction"
                    }),
                );
            }

            // Get description from contract components
            let description = self
                .contract_components
                .read_functions
                .iter()
                .chain(self.contract_components.write_functions.iter())
                .find(|f| {
                    let fn_name = f
                        .rust_signature
                        .split('(')
                        .next()
                        .unwrap_or("")
                        .split("fn")
                        .nth(1)
                        .unwrap_or("")
                        .trim()
                        .to_lowercase()
                        .replace("_", "");
                    fn_name == name.to_lowercase()
                })
                .map(|f| f.description.clone())
                .unwrap_or_else(|| format!("Calls the {} function", name));

            // Collect required fields after all properties are added
            let required_fields = properties.keys().cloned().collect::<Vec<_>>();

            tools.push(json!({
                "name": name,
                "description": description,
                "input_schema": {
                    "type": "object",
                    "properties": properties,
                    "required": required_fields
                }
            }));
        }

        let result = Value::Array(tools);

        // Write the tools to a JSON file
        write(
            "contract_tools.json",
            serde_json::to_string_pretty(&result)?,
        )?;

        Ok(result)
    }

    /// Calls a contract function with given parameters and value
    #[allow(dead_code)]
    async fn call_function(
        &self,
        function_name: &str,
        params: Option<Vec<String>>,
        value: Option<u128>,
    ) -> Result<()> {
        let functions = self
            .contract_instance
            .abi()
            .function(function_name)
            .ok_or_else(|| anyhow!("Function not found: {}", function_name))?;
        let func = functions
            .first()
            .ok_or_else(|| anyhow!("No function implementation found"))?;

        let provider = self.contract_instance.provider();
        let root_provider = provider.root();

        // Encode function call with parameters
        // Convert string parameters to DynSolValue
        let sol_params: Vec<DynSolValue> = params
            .unwrap_or_default()
            .iter()
            .map(|s| DynSolValue::String(s.clone()))
            .collect();

        let call_data = func
            .abi_encode_input_raw(&sol_params)
            .map_err(|e| anyhow!("Failed to encode parameters: {}", e))?;

        let tx_input = TransactionInput::new(Bytes::from(call_data));

        // Create transaction request
        let tx = TransactionRequest::default()
            .to(*self.contract_instance.address())
            .input(tx_input)
            .value(alloy::primitives::Uint::from(value.unwrap_or_default()));

        root_provider
            .call(&tx)
            .await
            .map_err(|e| anyhow!("Transaction failed: {}", e))?;

        Ok(())
    }
}

impl ContractInteraction<RootProvider<BoxTransport>> {
    pub fn new(interface: &str, contract_components: ContractComponents) -> Result<Self> {
        dotenv().ok();
        let address = Address::from_str(std::env::var("CONTRACT_ADDRESS").unwrap().as_str())?;
        let rpc_url = std::env::var("RPC_URL").expect("RPC_URL must be set");
        let rpc_url = Url::parse(&rpc_url).expect("Failed to parse RPC URL");
        let transport = http::Http::new(rpc_url);
        let boxed_transport = BoxTransport::new(transport);
        let client = RpcClient::new(boxed_transport, false);
        let provider = RootProvider::new(client);

        let human_readable_abi = Self::interface_to_human_readable(interface)?;

        let contract_instance = Self::build_contract(address, human_readable_abi, provider)?;

        Ok(Self {
            contract_instance,
            contract_components,
        })
    }

    fn interface_to_human_readable(interface: &str) -> Result<Value> {
        let mut signatures = Vec::new();

        // Match function declarations including 'external view' and memory keywords
        let fn_regex = Regex::new(
            r"function\s+(\w+)\s*\((.*?)\)(?:\s+(?:external|public))?\s*(?:(?:pure|view))?\s*(?:payable)?\s*(?:returns\s*\((.*?)\))?"
        ).unwrap();

        // Match error declarations
        let error_regex = Regex::new(r"error\s+(\w+)\s*\((.*?)\)").unwrap();

        for line in interface.lines() {
            let line = line.trim();

            // Process function declarations
            if let Some(caps) = fn_regex.captures(line) {
                let name = caps.get(1).map_or("", |m| m.as_str());
                let inputs = caps.get(2).map_or("", |m| m.as_str());
                let outputs = caps.get(3).map_or("", |m| m.as_str());

                let mut signature = format!("function {}", name);

                // Handle inputs
                if inputs.is_empty() {
                    signature.push_str("()");
                } else {
                    // Add memory/calldata keywords for string parameters
                    let processed_inputs = inputs
                        .split(',')
                        .map(|param| {
                            let param = param.trim();
                            if param.contains("string") {
                                if line.contains("external") {
                                    "string calldata".to_string()
                                } else {
                                    "string memory".to_string()
                                }
                            } else {
                                param.to_string()
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    signature.push_str(&format!("({})", processed_inputs));
                }

                // Add modifiers
                if line.contains("view") {
                    signature.push_str(" view");
                }
                if line.contains("payable") {
                    signature.push_str(" payable");
                }

                // Handle returns with memory keyword for string
                if !outputs.is_empty() {
                    let processed_outputs = if outputs.contains("string") {
                        "string memory"
                    } else {
                        outputs
                    };
                    signature.push_str(&format!(" returns ({})", processed_outputs));
                }

                signatures.push(signature);
            }

            // Process error declarations
            if let Some(caps) = error_regex.captures(line) {
                let name = caps.get(1).map_or("", |m| m.as_str());
                let params = caps.get(2).map_or("", |m| m.as_str());

                let signature = format!("error {}({})", name, params);
                signatures.push(signature);
            }
        }

        if signatures.is_empty() {
            return Err(anyhow!("No functions or errors found in interface"));
        }

        Ok(Value::Array(
            signatures.into_iter().map(Value::String).collect(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract_components::{ComponentDetails, ContractComponents};

    fn create_mock_contract_components() -> ContractComponents {
        ContractComponents {
            events: vec![],
            errors: vec![],
            storage_variables: vec![],
            read_functions: vec![
                ComponentDetails {
                    description: "Returns the current active prompt in the auction".to_string(),
                    rust_signature: "pub fn activePrompt(&self) -> Result<String, Error>"
                        .to_string(),
                    solidity_signature: "function activePrompt() view returns (string memory)"
                        .to_string(),
                },
                ComponentDetails {
                    description: "Returns the bid amount for a specific bidder".to_string(),
                    rust_signature: "pub fn bids(&self, bidder: Address) -> Result<U256, Error>"
                        .to_string(),
                    solidity_signature: "function bids(address bidder) view returns (uint256)"
                        .to_string(),
                },
            ],
            write_functions: vec![ComponentDetails {
                description: "Places a bid in the auction".to_string(),
                rust_signature: "pub fn bid(&mut self, prompt: String) -> Result<(), Error>"
                    .to_string(),
                solidity_signature: "function bid(string calldata prompt) payable".to_string(),
            }],
        }
    }

    #[test]
    fn test_payable_function_value_parameter() {
        struct MockProvider;
        impl Provider for MockProvider {
            fn root(&self) -> &RootProvider<BoxTransport> {
                unimplemented!("This is just a mock")
            }
        }

        let address = Address::from_slice(&[0u8; 20]);
        let provider = MockProvider;
        let contract_components = ContractComponents {
            events: vec![],
            errors: vec![],
            storage_variables: vec![],
            read_functions: vec![],
            write_functions: vec![ComponentDetails {
                description: "Places a bid".to_string(),
                rust_signature: "pub fn bid(&mut self, amount: String) -> Result<(), Error>"
                    .to_string(),
                solidity_signature: "function bid(string calldata) payable".to_string(),
            }],
        };

        let abi = serde_json::json!(["function bid(string calldata) payable"]);
        let contract = ContractInteraction::build_contract(address, abi, provider).unwrap();
        let interaction = ContractInteraction {
            contract_instance: contract,
            contract_components,
        };

        let tools = interaction
            .build_contract_claude_tool_definitions()
            .unwrap();
        let tools_array = tools.as_array().unwrap();

        // Check the bid function properties
        let bid_tool = &tools_array[0];
        let properties = bid_tool["input_schema"]["properties"].as_object().unwrap();
        let required = bid_tool["input_schema"]["required"].as_array().unwrap();

        // Verify value parameter exists
        assert!(properties.contains_key("value"));
        assert_eq!(properties["value"]["type"], "string");
        assert_eq!(
            properties["value"]["description"],
            "Amount of native currency to send with transaction"
        );

        // Verify value is in required fields
        let required_values: Vec<_> = required.iter().map(|v| v.as_str().unwrap()).collect();
        assert!(required_values.contains(&"value"));
    }

    #[test]
    fn test_build_contract_claude_tool_definitions() {
        struct MockProvider;
        impl Provider for MockProvider {
            fn root(&self) -> &RootProvider<BoxTransport> {
                unimplemented!("This is just a mock")
            }
        }

        let address = Address::from_slice(&[0u8; 20]);
        let provider = MockProvider;
        let contract_components = create_mock_contract_components();

        let abi = serde_json::json!([
            "function activePrompt() view returns (string memory)",
            "function bids(address bidder) view returns (uint256)",
            "function bid(string calldata prompt) payable"
        ]);

        let contract = ContractInteraction::build_contract(address, abi, provider).unwrap();
        let interaction = ContractInteraction {
            contract_instance: contract,
            contract_components,
        };

        let tools = interaction
            .build_contract_claude_tool_definitions()
            .unwrap();
        let tools_array = tools.as_array().unwrap();

        // Test number of tools matches number of functions in ABI
        assert_eq!(tools_array.len(), 3);

        // Find and test each tool by name
        let find_tool = |name: &str| {
            tools_array
                .iter()
                .find(|t| t["name"].as_str().unwrap() == name)
                .expect("Tool not found")
        };

        // Test activePrompt
        let active_prompt_tool = find_tool("activePrompt");
        assert_eq!(
            active_prompt_tool["description"],
            "Returns the current active prompt in the auction"
        );
        assert!(active_prompt_tool["input_schema"]["properties"]
            .as_object()
            .unwrap()
            .is_empty());

        // Test bids
        let bids_tool = find_tool("bids");
        assert_eq!(
            bids_tool["description"],
            "Returns the bid amount for a specific bidder"
        );
        let bids_props = bids_tool["input_schema"]["properties"].as_object().unwrap();
        assert!(bids_props["bidder"]["type"].as_str().unwrap() == "string");

        // Test bid
        let bid_tool = find_tool("bid");
        assert_eq!(bid_tool["description"], "Places a bid in the auction");
        let bid_props = bid_tool["input_schema"]["properties"].as_object().unwrap();
        assert!(bid_props.contains_key("prompt"));
        assert_eq!(bid_props["prompt"]["type"].as_str().unwrap(), "string");
    }

    #[test]
    fn test_interface_to_human_readable() {
        let interface = r#"
            /**
             * This file was automatically generated by Stylus and represents a Rust program.
             * For more information, please see [The Stylus SDK](https://github.com/OffchainLabs/stylus-sdk-rs).
             */

            // SPDX-License-Identifier: MIT-OR-APACHE-2.0
            pragma solidity ^0.8.23;

            interface IEnglishAuction {
                function activePrompt() external view returns (string memory);

                function getImage(uint256 image_id) external view returns (string memory);

                function getImageOwner(address owner) external view returns (uint256);

                function aiSeller() external view returns (address);

                function endAt() external view returns (uint256);

                function started() external view returns (bool);

                function ended() external view returns (bool);

                function highestBidder() external view returns (address);

                function highestBid() external view returns (uint256);

                function bids(address bidder) external view returns (uint256);

                function initialize(uint256 starting_bid) external;

                function start() external;

                function bid(string calldata prompt) external payable;

                function imageGenerationStatus(uint256 token_id, bool active) external;

                function withdraw() external;

                function end() external;

                error AlreadyInitialized();

                error AlreadyStarted();

                error NotSeller();

                error AuctionEnded();

                error BidTooLow();

                error NotStarted();

                error NotEnded();

                error UnAuthorizedUpdate();
            }
        "#;

        let result =
            ContractInteraction::<RootProvider<BoxTransport>>::interface_to_human_readable(
                interface,
            )
            .unwrap();

        let signatures = result.as_array().unwrap();

        // Check total number of signatures
        assert_eq!(signatures.len(), 24);

        // Check specific signatures exist
        assert!(signatures
            .iter()
            .any(|s| s == "function activePrompt() view returns (string memory)"));
        assert!(signatures
            .iter()
            .any(|s| s == "function getImage(uint256 image_id) view returns (string memory)"));
        assert!(signatures
            .iter()
            .any(|s| s == "function bid(string calldata) payable"));
        assert!(signatures.iter().any(|s| s == "error AlreadyInitialized()"));
    }

    #[test]
    fn test_interface_to_human_readable_empty() {
        let result =
            ContractInteraction::<RootProvider<BoxTransport>>::interface_to_human_readable("");
        assert!(result.is_err());
    }

    #[test]
    fn test_build_contract() {
        struct MockProvider;
        impl Provider for MockProvider {
            fn root(&self) -> &RootProvider<BoxTransport> {
                unimplemented!("This is just a mock")
            }
        }

        let address = Address::from_slice(&[0u8; 20]);
        let provider = MockProvider;

        let abi = serde_json::json!([
            "function activePrompt() view returns (string memory)",
            "function getImage(uint256 image_id) view returns (string memory)",
            "function getImageOwner(address owner) view returns (uint256)",
            "function aiSeller() view returns (address)",
            "function endAt() view returns (uint256)",
            "function started() view returns (bool)",
            "function ended() view returns (bool)",
            "function highestBidder() view returns (address)",
            "function highestBid() view returns (uint256)",
            "function bids(address bidder) view returns (uint256)",
            "function initialize(uint256 starting_bid)",
            "function start()",
            "function bid(string calldata)",
            "function imageGenerationStatus(uint256 token_id, bool active)",
            "function withdraw()",
            "function end()",
            "error AlreadyInitialized()",
            "error AlreadyStarted()",
            "error NotSeller()",
            "error AuctionEnded()",
            "error BidTooLow()",
            "error NotStarted()",
            "error NotEnded()",
            "error UnAuthorizedUpdate()",
        ]);

        let result = ContractInteraction::build_contract(address, abi, provider);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_contract_invalid_abi() {
        struct MockProvider;
        impl Provider for MockProvider {
            fn root(&self) -> &RootProvider<BoxTransport> {
                unimplemented!("This is just a mock")
            }
        }

        let address = Address::from_slice(&[0u8; 20]);
        let provider = MockProvider;

        let invalid_abi = serde_json::json!("invalid");

        let result = ContractInteraction::build_contract(address, invalid_abi, provider);
        assert!(result.is_err());
    }
}
