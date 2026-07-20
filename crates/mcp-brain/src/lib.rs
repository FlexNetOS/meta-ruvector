//! mcp-brain: MCP server for the RuVector Shared Brain
//!
//! Enables Claude Code sessions to share and discover learning across sessions.
//! Knowledge is stored as RVF cognitive containers with witness chains,
//! Ed25519 signatures, and differential privacy proofs.
//!
//! # MCP Tools (22)
//!
//! All 22 tools are registered in [`tools::McpBrainTools::list_tools`].
//!
//! ## Core (11)
//!
//! - **brain_share**: Share a learning with the collective
//! - **brain_search**: Semantic search across shared knowledge
//! - **brain_get**: Retrieve a specific memory with full provenance
//! - **brain_vote**: Quality-gate a memory (Bayesian update)
//! - **brain_transfer**: Apply learned priors cross-domain
//! - **brain_drift**: Check if shared knowledge has drifted
//! - **brain_partition**: Get knowledge partitioned by mincut topology
//! - **brain_list**: List recent memories by category/quality
//! - **brain_delete**: Delete own contribution
//! - **brain_status**: System health
//! - **brain_sync**: Sync local MicroLoRA weights with federated consensus
//!
//! ## Brainpedia — ADR-062 (6)
//!
//! - **brain_page_create**: Create a Draft page (reputation-gated)
//! - **brain_page_get**: Get a page with its delta log and evidence
//! - **brain_page_delta**: Submit a correction/extension/deprecation delta
//! - **brain_page_deltas**: List a page's modification history
//! - **brain_page_evidence**: Add verifiable evidence to a page
//! - **brain_page_promote**: Promote a Draft page to Canonical
//!
//! ## WASM Executable Nodes — ADR-063 (5)
//!
//! - **brain_node_list**: List published WASM nodes
//! - **brain_node_publish**: Publish a WASM node with conformance vectors
//! - **brain_node_get**: Get node metadata and conformance vectors
//! - **brain_node_wasm**: Download a node's WASM binary (base64)
//! - **brain_node_revoke**: Revoke a node (publisher only)
//!
//! # Usage
//!
//! ```no_run
//! use mcp_brain::McpBrainServer;
//!
//! #[tokio::main]
//! async fn main() {
//!     let server = McpBrainServer::new();
//!     server.run_stdio().await.expect("Server failed");
//! }
//! ```

pub mod client;
pub mod embed;
pub mod pipeline;
pub mod server;
pub mod tools;
pub mod types;

pub use server::McpBrainServer;
