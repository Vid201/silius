/// Entry point
pub mod entry_point {
    pub const ADDRESS: &str = "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789";
    pub const VERSION: &str = "0.6.0";
}

/// RPC error codes
pub mod rpc_error_codes {
    pub const VALIDATION: i32 = -32500;
    pub const PAYMASTER: i32 = -32501;
    pub const OPCODE: i32 = -32502;
    pub const EXPIRATION: i32 = -32503;
    pub const ENTITY_BANNED: i32 = -32504;
    pub const STAKE_TOO_LOW: i32 = -32505;
    pub const SIGNATURE_AGGREGATOR: i32 = -32506;
    pub const SIGNATURE: i32 = -32507;
    pub const EXECUTION: i32 = -32521;
    pub const USER_OPERATION_HASH: i32 = -32601;
    pub const SANITY_CHECK: i32 = -32602;
}

/// Entities
pub mod entities {
    pub const FACTORY: &str = "factory";
    pub const ACCOUNT: &str = "account";
    pub const PAYMASTER: &str = "paymaster";
}

/// Builder JSON-RPC Endpoints
pub const RELAY_ENDPOINTS: &[(&str, &str)] = &[
    ("flashbots", "https://relay.flashbots.net"),
    ("flashbots_goerli", "https://relay-goerli.flashbots.net"),
    ("builder0x69", "http://builder0x69.io/"),
    ("edennetwork", "https://api.edennetwork.io/v1/bundle"),
    ("beaverbuild", "https://rpc.beaverbuild.org/"),
    ("lightspeedbuilder", "https://rpc.lightspeedbuilder.info/"),
    ("eth-builder", "https://eth-builder.com/"),
    ("ultrasound", "https://relay.ultrasound.money/"),
    ("agnostic-relay", "https://agnostic-relay.net/"),
    ("relayoor-wtf", "https://relayooor.wtf/"),
    ("rsync-builder", "https://rsync-builder.xyz/"),
];
