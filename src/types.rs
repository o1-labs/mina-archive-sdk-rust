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
///
/// Only `address` is required. Omitting `token_id` makes the server default to
/// the MINA token, `wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf`, and
/// omitting `status` defaults it to `BlockStatusFilter::All`.
///
/// **A range-less query is not "everything".** The server caps the block scan
/// at its configured `BLOCK_RANGE_SIZE` (10,000 by default), and the SDL warns
/// that you can get a *partial result* if you do not specify both `from` and
/// `to`. Page through a wide history with explicit bounds.
///
/// GraphQL `Int` is signed **32-bit**, so the block-height bounds are `i32`.
/// A height computed from a `u64` now fails to compile instead of coming back
/// as a runtime GraphQL validation error. Output heights stay `i64` — widening
/// an output is safe and the server already sends values as `Int`.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventFilterOptionsInput {
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<BlockStatusFilter>,
    /// Inclusive lower block-height bound. `Int` is 32-bit — see the type doc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<i32>,
    /// Exclusive upper block-height bound. `Int` is 32-bit — see the type doc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<i32>,
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
    /// Filter from this block height, **inclusive**.
    pub fn from(mut self, from: i32) -> Self {
        self.from = Some(from);
        self
    }
    /// Filter to this block height, **exclusive**.
    pub fn to(mut self, to: i32) -> Self {
        self.to = Some(to);
        self
    }
}

/// Filter actions from a specific account.
///
/// Only `address` is required. Omitting `token_id` makes the server default to
/// the MINA token, `wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf`, and
/// omitting `status` defaults it to `BlockStatusFilter::All`.
///
/// **A range-less query is not "everything".** The server caps the block scan
/// at its configured `BLOCK_RANGE_SIZE` (10,000 by default), and the SDL warns
/// that you can get a *partial result* if you do not specify both `from` and
/// `to`. Page through a wide history with explicit bounds.
///
/// GraphQL `Int` is signed **32-bit**, so the block-height bounds are `i32`.
/// A height computed from a `u64` now fails to compile instead of coming back
/// as a runtime GraphQL validation error. Output heights stay `i64` — widening
/// an output is safe and the server already sends values as `Int`.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionFilterOptionsInput {
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<BlockStatusFilter>,
    /// Inclusive lower block-height bound. `Int` is 32-bit — see the type doc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<i32>,
    /// Exclusive upper block-height bound. `Int` is 32-bit — see the type doc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<i32>,
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
    /// Filter from this block height, **inclusive**.
    pub fn from(mut self, from: i32) -> Self {
        self.from = Some(from);
        self
    }
    /// Filter to this block height, **exclusive**.
    pub fn to(mut self, to: i32) -> Self {
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
///
/// GraphQL `Int` is signed **32-bit**, so the block-height bounds are `i32`.
/// A height computed from a `u64` now fails to compile instead of coming back
/// as a runtime GraphQL validation error. Output heights stay `i64` — widening
/// an output is safe and the server already sends values as `Int`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationKeyUpdateFilterInput {
    pub verification_key_hash: String,
    /// Inclusive lower block-height bound. `Int` is 32-bit — see the type doc.
    pub from: i32,
    /// Exclusive upper block-height bound. `Int` is 32-bit — see the type doc.
    pub to: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<BlockStatusFilter>,
}

impl VerificationKeyUpdateFilterInput {
    /// Search `[from, to)` for account updates that set `verification_key_hash`.
    pub fn new(verification_key_hash: impl Into<String>, from: i32, to: i32) -> Self {
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
///
/// GraphQL `Int` is signed **32-bit**, so the block-height bounds are `i32`.
/// A height computed from a `u64` now fails to compile instead of coming back
/// as a runtime GraphQL validation error. Output heights stay `i64` — widening
/// an output is safe and the server already sends values as `Int`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct BlockQueryInput {
    #[serde(rename = "blockHeight_gte", skip_serializing_if = "Option::is_none")]
    pub block_height_gte: Option<i32>,
    #[serde(rename = "blockHeight_lt", skip_serializing_if = "Option::is_none")]
    pub block_height_lt: Option<i32>,
    /// Inclusive lower bound, as an ISO-8601 / RFC 3339 instant.
    ///
    /// The server coerces this with JavaScript's `new Date(value).getTime()`.
    /// A value it cannot parse becomes `NaN`, which reaches SQL as the literal
    /// string `"NaN"` and **matches nothing without erroring** — the query
    /// returns HTTP 200 and an empty list. `"14/08/2023"` fails this way;
    /// `"Aug 14 2023"` parses but depends on the server's local timezone.
    ///
    /// Prefer [`BlockQueryInput::date_time_gte_from_unix_ms`], which cannot
    /// express an unparseable value.
    #[serde(rename = "dateTime_gte", skip_serializing_if = "Option::is_none")]
    pub date_time_gte: Option<String>,
    /// Exclusive upper bound, as an ISO-8601 / RFC 3339 instant.
    ///
    /// Carries the same silent-empty-result hazard as
    /// [`BlockQueryInput::date_time_gte`]. Prefer
    /// [`BlockQueryInput::date_time_lt_from_unix_ms`].
    #[serde(rename = "dateTime_lt", skip_serializing_if = "Option::is_none")]
    pub date_time_lt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "inBestChain")]
    pub in_best_chain: Option<bool>,
}

impl BlockQueryInput {
    /// Set the inclusive lower bound from Unix epoch milliseconds.
    ///
    /// Formats to ISO-8601 with a `Z` offset, which the server always parses
    /// to a finite number — so this cannot produce the silent empty result
    /// that a hand-written date string can.
    pub fn date_time_gte_from_unix_ms(mut self, ms: i64) -> Self {
        self.date_time_gte = Some(iso8601_from_unix_ms(ms));
        self
    }

    /// Set the exclusive upper bound from Unix epoch milliseconds.
    ///
    /// See [`BlockQueryInput::date_time_gte_from_unix_ms`].
    pub fn date_time_lt_from_unix_ms(mut self, ms: i64) -> Self {
        self.date_time_lt = Some(iso8601_from_unix_ms(ms));
        self
    }
}

/// Format Unix epoch milliseconds as `YYYY-MM-DDTHH:MM:SS.sssZ`.
///
/// Hand-rolled because the crate carries no `chrono`/`time` dependency and
/// this is the only place a date has to be built. Uses Howard Hinnant's
/// `civil_from_days`, which is exact for the whole proleptic Gregorian range.
fn iso8601_from_unix_ms(ms: i64) -> String {
    const MS_PER_DAY: i64 = 86_400_000;
    let days = ms.div_euclid(MS_PER_DAY);
    let mut rem = ms.rem_euclid(MS_PER_DAY);

    let millis = rem % 1000;
    rem /= 1000;
    let seconds = rem % 60;
    rem /= 60;
    let minutes = rem % 60;
    let hours = rem / 60;

    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}.{millis:03}Z")
}

/// Days since 1970-01-01 to a (year, month, day) civil date.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m as u32, d as u32)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct TransactionInfo {
    pub status: String,
    pub hash: String,
    pub memo: String,
    pub authorization_kind: String,
    pub sequence_number: i64,
    /// Element-nullable in the SDL (`[Int]!`), so a member may be `null` even
    /// though the list itself is always present.
    pub zkapp_account_update_ids: Vec<Option<i64>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct EventData {
    pub account_update_id: String,
    pub transaction_info: Option<TransactionInfo>,
    /// Element-nullable in the SDL (`[String]!`), so a member may be `null`
    /// even though the list itself is always present.
    pub data: Vec<Option<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ActionData {
    pub account_update_id: String,
    pub transaction_info: Option<TransactionInfo>,
    /// Element-nullable in the SDL (`[String]!`), so a member may be `null`
    /// even though the list itself is always present.
    pub data: Vec<Option<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct BlockInfo {
    pub height: i64,
    pub state_hash: String,
    pub parent_hash: String,
    pub ledger_hash: String,
    pub chain_status: String,
    /// Unix epoch **milliseconds** as a decimal string, e.g. `"1692054601000"`.
    ///
    /// **Not** ISO-8601 — an RFC 3339 parser rejects it, and reading it as
    /// seconds puts the block in 1970. Parse the integer first. Contrast
    /// [`Block::date_time`], a few fields away, which *is* ISO-8601.
    /// Unix time in **milliseconds** as a decimal string, e.g.
    /// `"1692054601000"`.
    ///
    /// Not ISO-8601 — see [`Block::date_time`], which is the other encoding
    /// this schema uses. Parsing this one with an RFC 3339 parser fails, and
    /// reading it as seconds puts the block in the wrong century.
    pub timestamp: String,
    pub global_slot_since_hardfork: i64,
    pub global_slot_since_genesis: i64,
    pub distance_from_max_block_height: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ActionStates {
    pub action_state_one: Option<String>,
    pub action_state_two: Option<String>,
    pub action_state_three: Option<String>,
    pub action_state_four: Option<String>,
    pub action_state_five: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct EventOutput {
    pub block_info: Option<BlockInfo>,
    pub event_data: Option<Vec<Option<EventData>>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ActionOutput {
    pub block_info: Option<BlockInfo>,
    pub transaction_info: Option<TransactionInfo>,
    pub action_data: Option<Vec<Option<ActionData>>>,
    pub action_state: ActionStates,
}

/// An applied account update that set a verification key.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
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
#[non_exhaustive]
pub struct MaxBlockHeightInfo {
    pub canonical_max_block_height: i64,
    pub pending_max_block_height: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct NetworkStateOutput {
    pub max_block_height: Option<MaxBlockHeightInfo>,
}

#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
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
#[non_exhaustive]
pub struct ZkAppCommand {
    pub hash: String,
    pub fee_payer: String,
    pub fee: String,
    pub memo: String,
    pub status: String,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
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
#[non_exhaustive]
pub struct BlockTransactions {
    pub coinbase: String,
    pub user_commands: Vec<UserCommand>,
    pub zkapp_commands: Vec<ZkAppCommand>,
    pub fee_transfer: Vec<FeeTransfer>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Block {
    pub block_height: i64,
    pub creator: String,
    pub state_hash: String,
    pub parent_hash: String,
    /// ISO-8601 / RFC 3339 instant, e.g. `"2023-08-14T23:10:01.000Z"`.
    ///
    /// The server derives it from the same archive column that
    /// [`BlockInfo::timestamp`] exposes raw, so the two describe the same
    /// instant in two different encodings: this one is ISO-8601, that one is
    /// Unix milliseconds as a decimal string, despite the identical Rust type.
    pub date_time: String,
    pub transactions: BlockTransactions,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The values in #13's evidence table, plus the epoch and a pre-epoch
    /// instant to exercise the floor-division path.
    #[test]
    fn iso8601_from_unix_ms_matches_javascript_to_iso_string() {
        let cases: &[(i64, &str)] = &[
            (1_691_971_200_000, "2023-08-14T00:00:00.000Z"),
            (1_692_054_601_000, "2023-08-14T23:10:01.000Z"),
            (0, "1970-01-01T00:00:00.000Z"),
            (-1, "1969-12-31T23:59:59.999Z"),
            (951_782_400_000, "2000-02-29T00:00:00.000Z"),
            (4_102_444_800_000, "2100-01-01T00:00:00.000Z"),
        ];
        for (ms, expected) in cases {
            assert_eq!(&iso8601_from_unix_ms(*ms), expected, "for {ms} ms");
        }
    }

    /// The point of the typed constructors: the string they emit is one the
    /// server's `new Date(v).getTime()` turns back into the same finite
    /// number, so the filter cannot silently match nothing.
    #[test]
    fn typed_constructors_round_trip_through_the_wire_format() {
        let input = BlockQueryInput::default()
            .date_time_gte_from_unix_ms(1_691_971_200_000)
            .date_time_lt_from_unix_ms(1_692_054_601_000);

        let json = serde_json::to_value(&input).unwrap();
        assert_eq!(json["dateTime_gte"], "2023-08-14T00:00:00.000Z");
        assert_eq!(json["dateTime_lt"], "2023-08-14T23:10:01.000Z");
    }
}
