use serde::{Deserialize, Serialize};

/// Filter for the consensus status of a block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BlockStatusFilter {
    All,
    Pending,
    Canonical,
}

/// Sort direction for block queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockSortBy {
    #[serde(rename = "BLOCKHEIGHT_ASC")]
    Asc,
    #[serde(rename = "BLOCKHEIGHT_DESC")]
    Desc,
}

/// Filter events from a specific account.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventFilterOptionsInput {
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<BlockStatusFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<i64>,
}

impl EventFilterOptionsInput {
    /// Start building a filter with the required `address`.
    pub fn for_address(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            ..Default::default()
        }
    }

    pub fn token_id(mut self, token_id: impl Into<String>) -> Self {
        self.token_id = Some(token_id.into());
        self
    }
    pub fn status(mut self, status: BlockStatusFilter) -> Self {
        self.status = Some(status);
        self
    }
    pub fn from(mut self, from: i64) -> Self {
        self.from = Some(from);
        self
    }
    pub fn to(mut self, to: i64) -> Self {
        self.to = Some(to);
        self
    }
}

/// Filter actions from a specific account.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionFilterOptionsInput {
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<BlockStatusFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_action_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_action_state: Option<String>,
}

impl ActionFilterOptionsInput {
    pub fn for_address(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            ..Default::default()
        }
    }
    pub fn token_id(mut self, v: impl Into<String>) -> Self {
        self.token_id = Some(v.into());
        self
    }
    pub fn status(mut self, status: BlockStatusFilter) -> Self {
        self.status = Some(status);
        self
    }
    pub fn from(mut self, from: i64) -> Self {
        self.from = Some(from);
        self
    }
    pub fn to(mut self, to: i64) -> Self {
        self.to = Some(to);
        self
    }
    pub fn from_action_state(mut self, v: impl Into<String>) -> Self {
        self.from_action_state = Some(v.into());
        self
    }
    pub fn end_action_state(mut self, v: impl Into<String>) -> Self {
        self.end_action_state = Some(v.into());
        self
    }
}

/// Filter for `verificationKeyUpdates`.
///
/// The block range is required, unlike the event and action filters: the
/// server bounds the span by its configured `BLOCK_RANGE_SIZE`. `from` is
/// inclusive and `to` is exclusive.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationKeyUpdateFilterInput {
    pub verification_key_hash: String,
    pub from: i64,
    pub to: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<BlockStatusFilter>,
}

impl VerificationKeyUpdateFilterInput {
    /// Search `[from, to)` for account updates that set `verification_key_hash`.
    pub fn new(verification_key_hash: impl Into<String>, from: i64, to: i64) -> Self {
        Self {
            verification_key_hash: verification_key_hash.into(),
            from,
            to,
            status: None,
        }
    }

    pub fn status(mut self, v: BlockStatusFilter) -> Self {
        self.status = Some(v);
        self
    }
}

/// Filter blocks by height, date, or canonical status.
#[derive(Debug, Clone, Default, Serialize)]
pub struct BlockQueryInput {
    #[serde(rename = "blockHeight_gte", skip_serializing_if = "Option::is_none")]
    pub block_height_gte: Option<i64>,
    #[serde(rename = "blockHeight_lt", skip_serializing_if = "Option::is_none")]
    pub block_height_lt: Option<i64>,
    #[serde(rename = "dateTime_gte", skip_serializing_if = "Option::is_none")]
    pub date_time_gte: Option<String>,
    #[serde(rename = "dateTime_lt", skip_serializing_if = "Option::is_none")]
    pub date_time_lt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "inBestChain")]
    pub in_best_chain: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionInfo {
    pub status: String,
    pub hash: String,
    pub memo: String,
    pub authorization_kind: String,
    pub sequence_number: i64,
    pub zkapp_account_update_ids: Vec<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventData {
    pub account_update_id: String,
    pub transaction_info: Option<TransactionInfo>,
    pub data: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionData {
    pub account_update_id: String,
    pub transaction_info: Option<TransactionInfo>,
    pub data: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockInfo {
    pub height: i64,
    pub state_hash: String,
    pub parent_hash: String,
    pub ledger_hash: String,
    pub chain_status: String,
    pub timestamp: String,
    pub global_slot_since_hardfork: i64,
    pub global_slot_since_genesis: i64,
    pub distance_from_max_block_height: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionStates {
    pub action_state_one: Option<String>,
    pub action_state_two: Option<String>,
    pub action_state_three: Option<String>,
    pub action_state_four: Option<String>,
    pub action_state_five: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventOutput {
    pub block_info: Option<BlockInfo>,
    pub event_data: Option<Vec<Option<EventData>>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionOutput {
    pub block_info: Option<BlockInfo>,
    pub transaction_info: Option<TransactionInfo>,
    pub action_data: Option<Vec<Option<ActionData>>>,
    pub action_state: ActionStates,
}

/// An applied account update that set a verification key.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationKeyUpdate {
    pub account_update_id: String,
    /// The account whose verification key was set.
    pub address: String,
    pub token_id: String,
    pub verification_key_hash: String,
    pub block_info: BlockInfo,
    pub transaction_info: TransactionInfo,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaxBlockHeightInfo {
    pub canonical_max_block_height: i64,
    pub pending_max_block_height: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkStateOutput {
    pub max_block_height: Option<MaxBlockHeightInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserCommand {
    pub hash: String,
    pub kind: String,
    pub from: String,
    pub to: String,
    pub amount: String,
    pub fee: String,
    pub memo: String,
    pub nonce: i64,
    pub status: String,
    #[serde(rename = "failureReason")]
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZkAppCommand {
    pub hash: String,
    pub fee_payer: String,
    pub fee: String,
    pub memo: String,
    pub status: String,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeeTransfer {
    pub recipient: String,
    pub fee: String,
    #[serde(rename = "type")]
    pub kind: String,
}

/// Transactions attached to a block.
///
/// Only `coinbase` is populated by a stock server. The other three fields
/// require `ENABLE_BLOCK_TRANSACTION_DETAILS=true`, which defaults to `false`,
/// and are empty otherwise. `Block::parent_hash` is `""` under the same flag.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockTransactions {
    pub coinbase: String,
    pub user_commands: Vec<UserCommand>,
    pub zkapp_commands: Vec<ZkAppCommand>,
    pub fee_transfer: Vec<FeeTransfer>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    pub block_height: i64,
    pub creator: String,
    pub state_hash: String,
    pub parent_hash: String,
    pub date_time: String,
    pub transactions: BlockTransactions,
}
