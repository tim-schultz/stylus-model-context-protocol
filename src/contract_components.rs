use anyhow::Result;
use log::info;
use regex::Regex;

#[derive(Debug)]
pub struct ComponentDetails {
    pub description: String,
    pub rust_signature: String,
    pub solidity_signature: String,
}

impl ComponentDetails {
    fn build_solidity_call(&self) -> String {
        let parts: Vec<&str> = self.solidity_signature.split_whitespace().collect();
        let name = parts.get(1).unwrap_or(&"");
        let params = parts.get(2..).unwrap_or(&[]);
        let params = params.join(" ");
        format!("{}({})", name, params)
    }
    fn parse_solidity_signature(interface: &str, rust_name: &str) -> String {
        let to_camel_case = |name: &str| -> String {
            let mut result = String::new();
            let mut parts = name.split('_');

            // First part is lowercase
            if let Some(first) = parts.next() {
                result.push_str(&first.to_lowercase());
            }

            // Rest is PascalCase
            for part in parts {
                if !part.is_empty() {
                    result.push_str(&(part[0..1].to_uppercase() + &part[1..]));
                }
            }

            result
        };

        let to_pascal_case = |name: &str| -> String {
            name.split('_')
                .filter(|s| !s.is_empty())
                .map(|s| s[0..1].to_uppercase() + &s[1..])
                .collect()
        };

        let is_error = interface.lines().any(|line| {
            line.trim().starts_with("error") && line.contains(&to_pascal_case(rust_name))
        });

        let name = if is_error {
            to_pascal_case(rust_name)
        } else {
            to_camel_case(rust_name)
        };

        interface
            .lines()
            .find(|line| {
                let line = line.trim();
                if is_error {
                    line.starts_with("error") && line.contains(&name)
                } else {
                    line.starts_with("function") && line.contains(&name)
                }
            })
            .map(|line| line.trim().trim_end_matches(';').to_string())
            .unwrap_or_default()
    }
}

#[derive(Debug, Default)]
pub struct ContractComponents {
    pub events: Vec<ComponentDetails>,
    pub errors: Vec<ComponentDetails>,
    pub storage_variables: Vec<ComponentDetails>,
    pub read_functions: Vec<ComponentDetails>,
    pub write_functions: Vec<ComponentDetails>,
}

impl ContractComponents {
    pub fn new(response: &str, interface: Option<&str>) -> Self {
        let mut components = Self::default();

        // Helper function to extract description and signature from a component block
        fn extract_details(block: &str) -> ComponentDetails {
            let desc_re = Regex::new(r"(?s)<description>\s*(.*?)\s*</description>").unwrap();
            let sig_re = Regex::new(r"(?s)<signature>\s*(.*?)\s*</signature>").unwrap();

            let description = desc_re
                .captures(block)
                .map(|cap| cap[1].trim().to_string())
                .unwrap_or_default();

            let rust_signature = sig_re
                .captures(block)
                .map(|cap| cap[1].trim().to_string())
                .unwrap_or_default();

            ComponentDetails {
                description,
                rust_signature,
                solidity_signature: String::new(),
            }
        }

        // Create regex patterns for each component with dot-all flag
        let event_re = Regex::new(r"(?s)<event>.*?</event>").unwrap();
        let error_re = Regex::new(r"(?s)<error>.*?</error>").unwrap();
        let storage_re = Regex::new(r"(?s)<storage_variable>.*?</storage_variable>").unwrap();
        let read_fn_re = Regex::new(r"(?s)<read_function>.*?</read_function>").unwrap();
        let write_fn_re = Regex::new(r"(?s)<write_function>.*?</write_function>").unwrap();

        // Extract components with details
        for cap in event_re.find_iter(response) {
            let mut details = extract_details(cap.as_str());
            if let Some(iface) = interface {
                details.solidity_signature =
                    ComponentDetails::parse_solidity_signature(iface, &details.rust_signature);
            }
            components.events.push(details);
        }

        for cap in error_re.find_iter(response) {
            let mut details = extract_details(cap.as_str());
            if let Some(iface) = interface {
                details.solidity_signature =
                    ComponentDetails::parse_solidity_signature(iface, &details.rust_signature);
            }
            components.errors.push(details);
        }

        for cap in storage_re.find_iter(response) {
            let mut details = extract_details(cap.as_str());
            if let Some(iface) = interface {
                details.solidity_signature =
                    ComponentDetails::parse_solidity_signature(iface, &details.rust_signature);
            }
            components.storage_variables.push(details);
        }

        for cap in read_fn_re.find_iter(response) {
            let mut details = extract_details(cap.as_str());
            if let Some(iface) = interface {
                details.solidity_signature =
                    ComponentDetails::parse_solidity_signature(iface, &details.rust_signature);
            }
            components.read_functions.push(details);
        }

        for cap in write_fn_re.find_iter(response) {
            let mut details = extract_details(cap.as_str());
            if let Some(iface) = interface {
                details.solidity_signature =
                    ComponentDetails::parse_solidity_signature(iface, &details.rust_signature);
            }
            components.write_functions.push(details);
        }
        components
    }

    pub fn generate_markdown(self, output_path: &str) -> Result<()> {
        // Create markdown content
        let mut md_content = String::new();

        md_content.push_str("# Contract Analysis\n\n");

        md_content.push_str(&format!("## Events ({})\n\n", self.events.len()));
        for event in &self.events {
            md_content.push_str(&format!(
                "### Description\n{}\n\n### Rust Signature\n```rust\n{}\n```\n\n### Solidity Signature\n```solidity\n{}\n```\n\n",
                event.description, event.rust_signature, event.solidity_signature
            ));
        }

        md_content.push_str(&format!("## Errors ({})\n\n", self.errors.len()));
        for error in &self.errors {
            md_content.push_str(&format!(
                "### Description\n{}\n\n### Rust Signature\n```rust\n{}\n```\n\n### Solidity Signature\n```solidity\n{}\n```\n\n",
                error.description, error.rust_signature, error.solidity_signature
            ));
        }

        md_content.push_str(&format!(
            "## Storage Variables ({})\n\n",
            self.storage_variables.len()
        ));
        for var in &self.storage_variables {
            md_content.push_str(&format!(
                "### Description\n{}\n\n### Rust Signature\n```rust\n{}\n```\n\n### Solidity Signature\n```solidity\n{}\n```\n\n",
                var.description, var.rust_signature, var.solidity_signature
            ));
        }

        md_content.push_str(&format!(
            "## Read Functions ({})\n\n",
            self.read_functions.len()
        ));
        for func in &self.read_functions {
            md_content.push_str(&format!(
                "### Description\n{}\n\n### Rust Signature\n```rust\n{}\n```\n\n### Solidity Signature\n```solidity\n{}\n```\n\n",
                func.description, func.rust_signature, func.solidity_signature
            ));
        }

        md_content.push_str(&format!(
            "## Write Functions ({})\n\n",
            self.write_functions.len()
        ));
        for func in &self.write_functions {
            md_content.push_str(&format!(
                "### Description\n{}\n\n### Rust Signature\n```rust\n{}\n```\n\n### Solidity Signature\n```solidity\n{}\n```\n\n",
                func.description, func.rust_signature, func.solidity_signature
            ));
        }

        std::fs::write(output_path, md_content)?;
        info!("Analysis saved to {}", output_path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{ops::Add, str::FromStr};

    use alloy::{
        primitives::{Address, U256},
        providers::{Provider, ProviderBuilder},
        sol,
    };

    use super::*;

    const SAMPLE_INTERFACE: &str = r#"
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

    #[tokio::test]
    async fn test_parse_function_signature() {
        sol! {
            #[allow(missing_docs)]
            #[sol(rpc)]
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
        }
        let rpc_url = "http://localhost:8547".parse().unwrap();
        let provider = ProviderBuilder::new().on_http(rpc_url);
        let auction = IEnglishAuction::new(
            Address::from_str("0xa6e41ffd769491a42a6e5ce453259b93983a22ef").unwrap(),
            provider,
        );
        let auction_result = auction
            .bid("Yoooo".to_string())
            .value(U256::from(3))
            .call()
            .await
            .unwrap();
        let details = ComponentDetails::parse_solidity_signature(SAMPLE_INTERFACE, "bid_too_low");
        assert_eq!(details, "error BidTooLow()");
    }

    #[test]
    fn test_parse_function_with_params() {
        let details = ComponentDetails::parse_solidity_signature(SAMPLE_INTERFACE, "get_image");
        assert_eq!(
            details,
            "function getImage(uint256 image_id) external view returns (string memory)"
        );
    }

    #[test]
    fn test_parse_error_signature() {
        let details =
            ComponentDetails::parse_solidity_signature(SAMPLE_INTERFACE, "already_initialized");
        assert_eq!(details, "error AlreadyInitialized()");
    }

    #[test]
    fn test_nonexistent_signature() {
        let details = ComponentDetails::parse_solidity_signature(SAMPLE_INTERFACE, "not_exists");
        assert_eq!(details, "");
    }

    #[test]
    fn test_contract_components_new() {
        let response = r#"
            <error>
                <description>Already initialized error</description>
                <signature>already_initialized</signature>
            </error>
            <read_function>
                <description>Get active prompt</description>
                <signature>active_prompt</signature>
            </read_function>
        "#;

        let components = ContractComponents::new(response, Some(SAMPLE_INTERFACE));

        assert_eq!(components.errors.len(), 1);
        assert_eq!(
            components.errors[0].solidity_signature,
            "error AlreadyInitialized()"
        );

        assert_eq!(components.read_functions.len(), 1);
        assert_eq!(
            components.read_functions[0].solidity_signature,
            "function activePrompt() external view returns (string memory)"
        );
    }
}
