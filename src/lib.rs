use libruskel::{Result, Ruskel};
use libtenx;

mod tools;

/// A contract for managing Rust API documentation using Ruskel
///
/// # Example
///
/// ```
/// use stylus_context_provider::StylusContract;
///
/// // Create a new contract with a custom API directory
/// let contract = StylusContract::new("api/docs");
///
/// // Access the underlying Ruskel instance
/// let ruskel = contract.ruskel();
/// println!("Ruskel instance initialized with path: {}", contract.api_dir());
///
/// // You can also create a contract with default settings
/// let default_contract = StylusContract::default();
/// assert_eq!(default_contract.api_dir(), "target");
/// ```
pub struct StylusContract {
    ruskel: Ruskel,
    api_dir: String,
}

impl StylusContract {
    /// Creates a new StylusContract with the specified API directory
    pub fn new(api_dir: &str) -> Self {
        let ruskel = Ruskel::new(api_dir);
        Self {
            ruskel,
            api_dir: api_dir.to_string(),
        }
    }

    /// Returns a reference to the Ruskel instance
    pub fn ruskel(&self) -> &Ruskel {
        &self.ruskel
    }

    /// Returns the API directory path
    pub fn api_dir(&self) -> &str {
        &self.api_dir
    }

    /// Generates a detailed analysis of the API directory
    pub fn analyze(&self) -> Result<String> {
        // Ensure crate is valid before proceeding
        self.ruskel.inspect(false)?;

        // Get the skeletonized version with full details
        let skeleton = self.ruskel.render(true, true, false)?;

        // Get the raw JSON representation
        let raw_json = self.ruskel.raw_json()?;

        // Combine all information into a detailed report
        Ok(format!(
            "
            Skeletonized Structure:\n\
            --------------------\n\
            {}",
            skeleton
        ))
    }
}

impl Default for StylusContract {
    fn default() -> Self {
        Self::new("target")
    }
}
