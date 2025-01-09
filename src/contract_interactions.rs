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
use serde_json::Value;
use url::Url;

pub struct ContractInteraction<P: Provider> {
    human_readable_abi: Value,
    #[allow(dead_code)]
    contract_instance: ContractInstance<Interface, P>,
}

impl ContractInteraction<RootProvider<BoxTransport>> {
    pub fn new(interface: &str) -> Result<Self> {
        dotenv().ok();
        let address = Address::from_str(std::env::var("CONTRACT_ADDRESS").unwrap().as_str())?;
        let rpc_url = std::env::var("RPC_URL").expect("RPC_URL must be set");
        let rpc_url = Url::parse(&rpc_url).expect("Failed to parse RPC URL");
        let transport = http::Http::new(rpc_url);
        let boxed_transport = BoxTransport::new(transport);
        let client = RpcClient::new(boxed_transport, false);
        let provider = RootProvider::new(client);

        let human_readable_abi = Self::interface_to_human_readable(interface)?;
        dbg!(human_readable_abi.clone());

        let contract_instance =
            ContractInteraction::build_contract(address, human_readable_abi.clone(), provider)?;

        Ok(Self {
            human_readable_abi,
            contract_instance,
        })
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

    pub fn get_abi(&self) -> &Value {
        &self.human_readable_abi
    }
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
        dbg!(&abi);
        let interface = Interface::new(abi);
        let instance = ContractInstance::new(address, provider, interface);
        Ok(instance)
    }
    fn interface_to_human_readable(interface: &str) -> Result<Value> {
        let mut signatures = Vec::new();

        // Match function declarations including 'external view' and memory keywords
        let fn_regex = Regex::new(
            r"function\s+(\w+)\s*\((.*?)\)(?:\s+(?:external|public))?\s*(?:(?:pure|view|payable))?\s*(?:returns\s*\((.*?)\))?"
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

                // Add view if present
                if line.contains("view") {
                    signature.push_str(" view");
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
        assert!(signatures.iter().any(|s| s == "error AlreadyInitialized()"));

        dbg!(&result);
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
