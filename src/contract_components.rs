use anyhow::Result;
use log::info;
use regex::Regex;

#[derive(Debug)]
pub struct ComponentDetails {
    pub description: String,
    pub rust_signature: String,
    pub solidity_signature: String,
}

// impl ComponentDetails {
//     fn build_dyn_solidity_call(&self) -> String {}
// }

#[derive(Debug, Default)]
pub struct ContractComponents {
    pub events: Vec<ComponentDetails>,
    pub errors: Vec<ComponentDetails>,
    pub storage_variables: Vec<ComponentDetails>,
    pub read_functions: Vec<ComponentDetails>,
    pub write_functions: Vec<ComponentDetails>,
}

impl ContractComponents {
    pub fn new(response: &str, _interface: Option<&str>) -> Self {
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
            let details = extract_details(cap.as_str());
            components.events.push(details);
        }

        for cap in error_re.find_iter(response) {
            let details = extract_details(cap.as_str());
            components.errors.push(details);
        }

        for cap in storage_re.find_iter(response) {
            let details = extract_details(cap.as_str());

            components.storage_variables.push(details);
        }

        for cap in read_fn_re.find_iter(response) {
            let details = extract_details(cap.as_str());
            components.read_functions.push(details);
        }

        for cap in write_fn_re.find_iter(response) {
            let details = extract_details(cap.as_str());
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
