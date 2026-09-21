use std::fmt;

/// Errors returned by the Mina Archive SDK.
///
/// `#[non_exhaustive]`, so a `match` needs a `_` arm. A new failure mode — a
/// new server error class, a new transport condition — must not be an SDK
/// major release, and adding a variant to an exhaustively matched enum is
/// breaking.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The GraphQL endpoint returned one or more errors. Not retried.
    ///
    /// Note `status` is almost always 200: the API answers every GraphQL-level
    /// error with HTTP 200 and a populated `errors` array. `extensions.status`
    /// is a payload field, not the HTTP status.
    #[error("GraphQL error in {query_name}: {messages}")]
    Graphql {
        query_name: String,
        status: reqwest::StatusCode,
        messages: String,
        errors: Vec<GraphqlErrorEntry>,
    },

    /// The server's rate limiter rejected the request with HTTP 429.
    ///
    /// This is the **only** non-200 the API emits under normal operation, so
    /// unlike most 4xx it unambiguously means "slow down", and it is the one
    /// case where retrying the identical request is correct.
    #[error("rate limited on {query_name}: {messages}")]
    RateLimited {
        query_name: String,
        /// From the `retry-after` header, when the server sends one.
        retry_after: Option<std::time::Duration>,
        /// From `x-ratelimit-limit`.
        limit: Option<u64>,
        /// From `x-ratelimit-remaining`.
        remaining: Option<u64>,
        messages: String,
    },

    /// A non-2xx response that is not GraphQL-shaped.
    ///
    /// The usual cause is a URL pointing somewhere the server does not serve:
    /// the endpoint is the root path `/`, and `/graphql` returns 404 with an
    /// HTML or plain-text body.
    #[error("unexpected HTTP {status} in {query_name}: {body}")]
    UnexpectedStatus {
        query_name: String,
        status: reqwest::StatusCode,
        body: String,
    },

    /// Failed to connect after exhausting all retry attempts.
    #[error("failed to execute {query_name} after {attempts} attempts: {source}")]
    Connection {
        query_name: String,
        attempts: u32,
        #[source]
        source: reqwest::Error,
    },

    /// A required field was missing in an otherwise-successful response —
    /// most likely a schema mismatch.
    #[error("missing field '{field}' in {query_name} response")]
    MissingField { query_name: String, field: String },

    /// Failed to deserialize the response body.
    #[error("failed to decode {query_name} response: {source}")]
    Decode {
        query_name: String,
        #[source]
        source: serde_json::Error,
    },

    /// Currency subtraction would underflow.
    #[error("currency underflow: {0} - {1} would be negative")]
    CurrencyUnderflow(u64, u64),

    /// String could not be parsed as a Mina/nanomina currency value.
    #[error("invalid currency format: {0}")]
    InvalidCurrency(String),
}

/// Contract error codes published by Archive-Node-API in `extensions.code`.
///
/// These are the supported way to branch on a failure. Message text is
/// deliberately minimal — the server blocks GraphQL field suggestions — and
/// carries no stability promise, so do not match on it.
pub mod codes {
    /// Range exceeds `BLOCK_RANGE_SIZE`. Narrow it; retrying unchanged fails.
    pub const BLOCK_RANGE_ERROR: &str = "BLOCK_RANGE_ERROR";
    /// The action state is not in the archive.
    pub const ACTION_STATE_NOT_FOUND: &str = "ACTION_STATE_NOT_FOUND";
    /// The action state falls outside the requested block range.
    pub const ACTION_STATE_OUT_OF_RANGE: &str = "ACTION_STATE_OUT_OF_RANGE";
    /// Too many requests. See [`crate::Error::RateLimited`].
    pub const RATE_LIMITED: &str = "RATE_LIMITED";
}

/// One entry from a GraphQL response's `errors` array.
///
/// `extensions.code` is the API's intended discriminator. It publishes four
/// codes as contract — `ACTION_STATE_NOT_FOUND`, `ACTION_STATE_OUT_OF_RANGE`,
/// `BLOCK_RANGE_ERROR` and `RATE_LIMITED` — and deliberately keeps the message
/// text minimal, so matching on the code is the supported way to branch.
///
/// Every field but `message` is optional, and `code` in particular must stay
/// optional: the server runs `maskedErrors: { isDev: false }`, so an
/// unexpected error arrives with a generic message and no `extensions` at all.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct GraphqlErrorEntry {
    pub message: String,
    /// Shortcut for `extensions.code`, lifted during decoding. `None` for a
    /// masked error.
    #[serde(skip)]
    pub code: Option<String>,
    /// Response path to the field that failed, when the server reports one.
    #[serde(default)]
    pub path: Option<Vec<serde_json::Value>>,
    /// Query source locations, when the server reports them.
    #[serde(default)]
    pub locations: Option<Vec<serde_json::Value>>,
    /// The whole `extensions` object, kept verbatim so callers can read fields
    /// beyond `code` without waiting for an SDK release.
    #[serde(default)]
    pub extensions: Option<serde_json::Value>,
}

impl GraphqlErrorEntry {
    /// Lift `extensions.code` into the `code` shortcut after deserialising.
    pub(crate) fn with_lifted_code(mut self) -> Self {
        self.code = self
            .extensions
            .as_ref()
            .and_then(|e| e.get("code"))
            .and_then(|c| c.as_str())
            .map(str::to_string);
        self
    }
}

impl fmt::Display for GraphqlErrorEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error {
    /// The `extensions.code` values carried by a [`Error::Graphql`], in order.
    ///
    /// Empty for every other variant, and for masked errors, which arrive with
    /// no `extensions` at all — so an empty result means "no code was sent",
    /// never "no error occurred".
    pub fn graphql_codes(&self) -> Vec<&str> {
        match self {
            Error::Graphql { errors, .. } => {
                errors.iter().filter_map(|e| e.code.as_deref()).collect()
            }
            _ => Vec::new(),
        }
    }

    /// True when any entry carries the given `extensions.code`.
    pub fn has_graphql_code(&self, code: &str) -> bool {
        self.graphql_codes().contains(&code)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
