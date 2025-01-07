use anyhow::Result;
use regex::Regex;

#[derive(Debug)]
pub struct ComponentDetails {
    pub description: String,
    pub signature: String,
}

#[derive(Debug)]
pub struct ContractComponents {
    pub events: Vec<ComponentDetails>,
    pub errors: Vec<ComponentDetails>,
    pub storage_variables: Vec<ComponentDetails>,
    pub read_functions: Vec<ComponentDetails>,
    pub write_functions: Vec<ComponentDetails>,
}

impl ContractComponents {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            errors: Vec::new(),
            storage_variables: Vec::new(),
            read_functions: Vec::new(),
            write_functions: Vec::new(),
        }
    }

    pub fn parse_response(response: &str) -> Result<Self> {
        let mut components = Self::new();

        // Helper function to extract description and signature from a component block
        fn extract_details(block: &str) -> ComponentDetails {
            let desc_re = Regex::new(r"(?s)<description>\s*(.*?)\s*</description>").unwrap();
            let sig_re = Regex::new(r"(?s)<signature>\s*(.*?)\s*</signature>").unwrap();

            let description = desc_re
                .captures(block)
                .map(|cap| cap[1].trim().to_string())
                .unwrap_or_default();

            let signature = sig_re
                .captures(block)
                .map(|cap| cap[1].trim().to_string())
                .unwrap_or_default();

            ComponentDetails {
                description,
                signature,
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
            components.events.push(extract_details(cap.as_str()));
        }

        for cap in error_re.find_iter(response) {
            components.errors.push(extract_details(cap.as_str()));
        }

        for cap in storage_re.find_iter(response) {
            components
                .storage_variables
                .push(extract_details(cap.as_str()));
        }

        for cap in read_fn_re.find_iter(response) {
            components
                .read_functions
                .push(extract_details(cap.as_str()));
        }

        for cap in write_fn_re.find_iter(response) {
            components
                .write_functions
                .push(extract_details(cap.as_str()));
        }

        Ok(components)
    }
}
