use std::time::Duration;

use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use tracing::{debug, warn};

use crate::error::{Error, GraphqlErrorEntry, Result};
use crate::queries;
use crate::types::*;

/// Configuration for the Archive Node client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// GraphQL endpoint URL.
    pub graphql_uri: String,
    /// Total number of attempts (including the initial try). Must be ≥ 1.
    pub retries: u32,
    /// Delay between retries.
    pub retry_delay: Duration,
    /// Per-request HTTP timeout.
    pub timeout: Duration,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            graphql_uri: "http://localhost:8080/".to_string(),
            retries: 3,
            retry_delay: Duration::from_secs(5),
            timeout: Duration::from_secs(30),
        }
    }
}

/// Client for querying a Mina Archive Node GraphQL endpoint.
///
/// # Examples
/// ```no_run
/// # async fn example() -> mina_archive_sdk::Result<()> {
/// use mina_archive_sdk::{ArchiveClient, BlockStatusFilter, EventFilterOptionsInput};
///
/// let client = ArchiveClient::new("https://archive.example/");
/// let events = client.get_events(
///     EventFilterOptionsInput::for_address("B62q...")
///         .status(BlockStatusFilter::Canonical),
/// ).await?;
/// # let _ = events;
/// # Ok(())
/// # }
/// ```
pub struct ArchiveClient {
    config: ClientConfig,
    http: reqwest::Client,
}

impl Default for ArchiveClient {
    fn default() -> Self {
        Self::with_config(ClientConfig::default())
    }
}

impl ArchiveClient {
    /// Build a client targeting `graphql_uri` with default retry/timeout.
    pub fn new(graphql_uri: &str) -> Self {
        Self::with_config(ClientConfig {
            graphql_uri: graphql_uri.to_string(),
            ..Default::default()
        })
    }

    /// Build a client with explicit configuration.
    ///
    /// # Panics
    /// Panics if `retries == 0` or `timeout` is zero.
    pub fn with_config(config: ClientConfig) -> Self {
        assert!(config.retries >= 1, "retries must be at least 1");
        assert!(
            !config.timeout.is_zero(),
            "timeout must be greater than zero"
        );
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("failed to build HTTP client");
        Self { config, http }
    }

    /// GraphQL endpoint URI.
    pub fn graphql_uri(&self) -> &str {
        &self.config.graphql_uri
    }

    /// Builder for arbitrary GraphQL queries with optional variables/name.
    pub fn query<'a>(&'a self, query: &'a str) -> QueryBuilder<'a> {
        QueryBuilder {
            client: self,
            query,
            variables: None,
            name: None,
        }
    }

    /// Run an arbitrary GraphQL query through the retry path. Returns the
    /// response's `data` field as a `serde_json::Value`. Prefer the typed
    /// methods below for the four built-in queries.
    pub async fn execute_query(
        &self,
        query: &str,
        variables: Option<Value>,
        query_name: &str,
    ) -> Result<Value> {
        let mut payload = json!({ "query": query });
        if let Some(vars) = variables {
            payload["variables"] = vars;
        }

        let mut last_err: Option<reqwest::Error> = None;

        for attempt in 1..=self.config.retries {
            debug!(
                query_name,
                attempt,
                max = self.config.retries,
                "GraphQL request"
            );

            match self
                .http
                .post(&self.config.graphql_uri)
                .json(&payload)
                .send()
                .await
            {
                Ok(resp) => {
                    // Read the status and headers before the body is consumed;
                    // resp.json() takes ownership and used to take the only
                    // record of what the server actually said with it (#8).
                    let status = resp.status();
                    let rate = RateLimitHeaders::from(resp.headers());

                    if status.is_server_error() || status == reqwest::StatusCode::REQUEST_TIMEOUT {
                        warn!(query_name, attempt, %status, "retryable HTTP error");
                        last_err = Some(resp.error_for_status().unwrap_err());
                        if attempt < self.config.retries {
                            tokio::time::sleep(self.config.retry_delay).await;
                        }
                        continue;
                    }

                    // 429 is the only non-200 the API emits under normal
                    // operation, so it unambiguously means "slow down" and is
                    // the one status where retrying unchanged is correct.
                    // Honour retry-after when the server sends it.
                    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        let text = resp.text().await.unwrap_or_default();
                        let messages = graphql_messages_from_body(&text)
                            .unwrap_or_else(|| "Too many requests.".to_string());
                        warn!(query_name, attempt, ?rate.retry_after, "rate limited");

                        if attempt < self.config.retries {
                            tokio::time::sleep(rate.retry_after.unwrap_or(self.config.retry_delay))
                                .await;
                            continue;
                        }
                        return Err(Error::RateLimited {
                            query_name: query_name.to_string(),
                            retry_after: rate.retry_after,
                            limit: rate.limit,
                            remaining: rate.remaining,
                            messages,
                        });
                    }

                    // Any other non-2xx is not retried, and is reported by its
                    // status rather than as a decode or missing-field failure.
                    if !status.is_success() {
                        let text = resp.text().await.unwrap_or_default();
                        if let Some(messages) = graphql_messages_from_body(&text) {
                            let errors = graphql_entries_from_body(&text);
                            return Err(Error::Graphql {
                                query_name: query_name.to_string(),
                                status,
                                messages,
                                errors,
                            });
                        }
                        return Err(Error::UnexpectedStatus {
                            query_name: query_name.to_string(),
                            status,
                            body: truncate(&text, 200),
                        });
                    }

                    let body: Value = match resp.json().await {
                        Ok(b) => b,
                        Err(e) => {
                            warn!(query_name, attempt, error = %e, "failed to parse JSON");
                            last_err = Some(e);
                            if attempt < self.config.retries {
                                tokio::time::sleep(self.config.retry_delay).await;
                            }
                            continue;
                        }
                    };

                    // An empty `errors` array is not an error. Testing only for the
                    // array's presence turned a perfectly good response into a failure
                    // whose message was the empty string (#11).
                    if let Some(errors) = body
                        .get("errors")
                        .and_then(|e| e.as_array())
                        .filter(|arr| !arr.is_empty())
                    {
                        // Deserialise the whole entry rather than hand-picking
                        // `message`, so `extensions.code`, `path` and `locations`
                        // survive (#9). A masked error carries no extensions at
                        // all, and an entry that will not deserialise still has to
                        // produce something, so fall back to the message alone.
                        let entries: Vec<GraphqlErrorEntry> = errors
                            .iter()
                            .map(|e| {
                                serde_json::from_value::<GraphqlErrorEntry>(e.clone())
                                    .unwrap_or_else(|_| GraphqlErrorEntry {
                                        message: e
                                            .get("message")
                                            .and_then(|m| m.as_str())
                                            .unwrap_or("unknown error")
                                            .to_string(),
                                        code: None,
                                        path: None,
                                        locations: None,
                                        extensions: None,
                                    })
                                    .with_lifted_code()
                            })
                            .collect();
                        let messages = entries
                            .iter()
                            .map(|e| e.message.as_str())
                            .collect::<Vec<_>>()
                            .join("; ");
                        return Err(Error::Graphql {
                            query_name: query_name.to_string(),
                            status,
                            messages,
                            errors: entries,
                        });
                    }

                    return Ok(body
                        .get("data")
                        .cloned()
                        .unwrap_or(Value::Object(Default::default())));
                }
                Err(e) => {
                    warn!(query_name, attempt, error = %e, "transport error");
                    last_err = Some(e);
                }
            }

            if attempt < self.config.retries {
                tokio::time::sleep(self.config.retry_delay).await;
            }
        }

        Err(Error::Connection {
            query_name: query_name.to_string(),
            attempts: self.config.retries,
            source: last_err.expect("at least one attempt must have been made"),
        })
    }

    // -- Typed queries --

    /// Query archived events for a zkApp account.
    pub async fn get_events(&self, input: EventFilterOptionsInput) -> Result<Vec<EventOutput>> {
        let data = self
            .execute_query(
                queries::EVENTS_QUERY,
                Some(json!({ "input": input })),
                "get_events",
            )
            .await?;
        decode_field(&data, "events", "get_events")
    }

    /// Query archived actions for a zkApp account.
    pub async fn get_actions(&self, input: ActionFilterOptionsInput) -> Result<Vec<ActionOutput>> {
        let data = self
            .execute_query(
                queries::ACTIONS_QUERY,
                Some(json!({ "input": input })),
                "get_actions",
            )
            .await?;
        decode_field(&data, "actions", "get_actions")
    }

    /// Archive node's max canonical / pending block heights.
    pub async fn get_network_state(&self) -> Result<NetworkStateOutput> {
        let data = self
            .execute_query(queries::NETWORK_STATE_QUERY, None, "get_network_state")
            .await?;
        decode_field(&data, "networkState", "get_network_state")
    }

    /// Query blocks by height/date range and chain status.
    ///
    /// Transaction detail is only populated when the server sets
    /// `ENABLE_BLOCK_TRANSACTION_DETAILS=true`, which **defaults to `false`**.
    /// Against a stock server every returned block has `parent_hash == ""` and
    /// empty `user_commands`, `zkapp_commands` and `fee_transfer`, while
    /// `coinbase` **is** populated — so the response looks healthy and is
    /// easily mistaken for an empty chain or an SDK bug.
    pub async fn get_blocks(&self, opts: GetBlocksOptions) -> Result<Vec<Block>> {
        let vars = json!({
            "query": opts.query,
            "limit": opts.limit,
            "sortBy": opts.sort_by,
        });
        let data = self
            .execute_query(queries::BLOCKS_QUERY, Some(vars), "get_blocks")
            .await?;
        decode_field(&data, "blocks", "get_blocks")
    }

    /// Find applied account updates that set a given verification key.
    ///
    /// The block range is required and the server caps its width, so walk a
    /// wide history in pages rather than in one call.
    pub async fn get_verification_key_updates(
        &self,
        input: VerificationKeyUpdateFilterInput,
    ) -> Result<Vec<VerificationKeyUpdate>> {
        let data = self
            .execute_query(
                queries::VERIFICATION_KEY_UPDATES_QUERY,
                Some(json!({ "input": input })),
                "get_verification_key_updates",
            )
            .await?;
        decode_field(
            &data,
            "verificationKeyUpdates",
            "get_verification_key_updates",
        )
    }
}

/// Options for [`ArchiveClient::get_blocks`]. All fields are optional.
#[derive(Debug, Clone, Default)]
pub struct GetBlocksOptions {
    pub query: Option<BlockQueryInput>,
    pub limit: Option<i64>,
    pub sort_by: Option<BlockSortBy>,
}

/// Builder returned by [`ArchiveClient::query`].
#[must_use = "call .send() to execute the query"]
pub struct QueryBuilder<'a> {
    client: &'a ArchiveClient,
    query: &'a str,
    variables: Option<Value>,
    name: Option<&'a str>,
}

impl<'a> QueryBuilder<'a> {
    pub fn variables(mut self, variables: Value) -> Self {
        self.variables = Some(variables);
        self
    }
    pub fn name(mut self, name: &'a str) -> Self {
        self.name = Some(name);
        self
    }
    pub async fn send(self) -> Result<Value> {
        self.client
            .execute_query(self.query, self.variables, self.name.unwrap_or("custom"))
            .await
    }
}

fn decode_field<T: DeserializeOwned>(data: &Value, field: &str, query_name: &str) -> Result<T> {
    let v = data
        .get(field)
        .filter(|v| !v.is_null())
        .ok_or_else(|| Error::MissingField {
            query_name: query_name.to_string(),
            field: field.to_string(),
        })?;
    serde_json::from_value(v.clone()).map_err(|source| Error::Decode {
        query_name: query_name.to_string(),
        source,
    })
}

/// The three rate-limit headers the server sends alongside a 429.
#[derive(Debug, Default)]
struct RateLimitHeaders {
    retry_after: Option<Duration>,
    limit: Option<u64>,
    remaining: Option<u64>,
}

impl From<&reqwest::header::HeaderMap> for RateLimitHeaders {
    fn from(headers: &reqwest::header::HeaderMap) -> Self {
        let num =
            |name: &str| -> Option<u64> { headers.get(name)?.to_str().ok()?.trim().parse().ok() };
        Self {
            // The server sends delay-seconds. The HTTP-date form of
            // retry-after is legal but is not what this API emits, so an
            // unparseable value yields None rather than a wrong duration.
            retry_after: num("retry-after").map(Duration::from_secs),
            limit: num("x-ratelimit-limit"),
            remaining: num("x-ratelimit-remaining"),
        }
    }
}

/// Join the `message` fields of a GraphQL-shaped body, if it is one.
///
/// Returns None when the body is not JSON, or carries no non-empty `errors`
/// array — which is how a 404 HTML page is told apart from a real GraphQL
/// response that happens to arrive with a non-200 status.
fn graphql_messages_from_body(text: &str) -> Option<String> {
    let entries = graphql_entries_from_body(text);
    if entries.is_empty() {
        return None;
    }
    Some(
        entries
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("; "),
    )
}

/// Decode the `errors` array of a GraphQL-shaped body, or an empty vector.
fn graphql_entries_from_body(text: &str) -> Vec<GraphqlErrorEntry> {
    serde_json::from_str::<Value>(text)
        .ok()
        .as_ref()
        .and_then(|v| v.get("errors"))
        .and_then(|e| e.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|e| serde_json::from_value::<GraphqlErrorEntry>(e.clone()).ok())
                .map(GraphqlErrorEntry::with_lifted_code)
                .collect()
        })
        .unwrap_or_default()
}

/// Shorten a body for inclusion in an error message.
fn truncate(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= max {
        return s.to_string();
    }
    let head: String = s.chars().take(max).collect();
    format!("{head}…")
}
