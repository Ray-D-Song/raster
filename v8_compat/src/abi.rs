//! Single source of truth for the Node 24 / ABI 137 compatibility profile.

/// `process.versions.modules` and `node_module.nm_version`.
pub const NODE_MODULE_VERSION: i32 = 137;

/// `process.version` without the leading `v`.
pub const NODE_VERSION: &str = "24.3.0";

/// `process.versions.node`.
pub const NODE_VERSIONS_NODE: &str = NODE_VERSION;

/// `process.versions.v8` for Node 24.3.0.
pub const NODE_VERSIONS_V8: &str = "13.5.233.11-node.32";

/// `napi_get_node_version().version` encoding for Node 24.3.0.
pub const NAPI_NODE_VERSION_U32: u32 = 0x18030000;

/// `napi_get_node_version().napi_version`.
pub const NAPI_API_VERSION: u32 = 9;

/// Loader ABI checksum label used by compat probes.
pub const ABI_PROFILE_LABEL: &str = "node24-abi137";
