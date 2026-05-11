use std::fmt;

/// Errors returned by the Mina Archive SDK.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The GraphQL endpoint returned one or more errors. Not retried.
    #[error("GraphQL error in {query_name}: {messages}")]
    Graphql {
        query_name: String,
        messages: String,
        errors: Vec<GraphqlErrorEntry>,
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

/// One entry from a GraphQL response's `errors` array.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct GraphqlErrorEntry {
    pub message: String,
}

impl fmt::Display for GraphqlErrorEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
