//! A GraphQL response that may carry data and errors at the same time.

use crate::error::GraphqlErrorEntry;

/// The result of one GraphQL call, keeping `data` and `errors` together.
///
/// HTTP 200 carrying **both** a partial `data` payload and a non-empty
/// `errors` array is a normal GraphQL outcome for field-level nulls, and is
/// reachable against this API because the root lists and most of their fields
/// are nullable. The strict methods on [`crate::ArchiveClient`] report that as
/// a total failure, which makes the rows that did succeed unrecoverable; the
/// `*_with_errors` methods return this instead.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Response<T> {
    /// The decoded payload. `None` only when the server sent `"data": null`.
    pub data: Option<T>,
    /// Errors reported alongside the data. Empty on a clean success.
    pub errors: Vec<GraphqlErrorEntry>,
    /// The HTTP status the response arrived with. Almost always 200 — this
    /// API answers GraphQL-level errors with 200 and a populated `errors`.
    pub status: reqwest::StatusCode,
}

impl<T> Response<T> {
    /// True when data and errors arrived together.
    pub fn is_partial(&self) -> bool {
        self.data.is_some() && !self.errors.is_empty()
    }

    /// The error messages, joined — the same text the strict methods report.
    pub fn messages(&self) -> String {
        self.errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// The `extensions.code` values carried by the errors, in order.
    pub fn codes(&self) -> Vec<&str> {
        self.errors
            .iter()
            .filter_map(|e| e.code.as_deref())
            .collect()
    }
}
