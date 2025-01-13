# WIP/MVP
## Example of A Stylus Smart Contract Model Context Protocol

This example demonstrates how to convert a stylus smart contract into a Claude desktop compatible model context protocol server. The tools have been generated using `src/contract_components.rs`. Contract interactions and smart contract function calls are handled by `src/contract_interactions.rs`. 

[This](https://github.com/tim-schultz/stylus-ai-image-auction) is the stylus smart contract used for this MVP.

### Tools

An example of the generated tool definitions can be found in `read_write_contract_tools.json`

## How to Build and Run Example Locally

### Prerequisites
- macOS (will handle Windows in the future)
- The latest version of Claude Desktop installed
- Install [Rust](https://www.rust-lang.org/tools/install)

### Build and Install Binary
```bash
cargo install --path .
```
This will build the binary and install it to your local cargo bin directory. Later you will need to configure Claude Desktop to use this binary.
### Configure Claude Desktop

If you are using macOS, open the `claude_desktop_config.json` file in a text editor:
```bash
code ~/Library/Application\ Support/Claude/claude_desktop_config.json
```

Modify the `claude_desktop_config.json` file to include the following configuration:
(replace YOUR_USERNAME with your actual username):
```json
{
  "mcpServers": {
    "stylus_smart_contract": {
      "command": "/Users/timschultz/.cargo/bin/context-server",
      "env": {
        "CONTRACT_ADDRESS": "0x0",
        "RPC_URL": "http://localhost:8547",
        "CONTRACT_DIRECTORY": "/Users/stylus-smart-contract-to-interact-with/",
        "PRIVATE_KEY": "0x0",
        "ANTHROPIC_API_KEY_RS": "sk-ant-...."
      }
    }
  }
}
```
Save the file, and restart Claude Desktop.