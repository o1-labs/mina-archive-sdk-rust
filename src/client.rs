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
/// let client = ArchiveClient::new("https://archive.example/graphql");
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
                    let status = resp.status();
                    if status.is_server_error() || status == reqwest::StatusCode::REQUEST_TIMEOUT {
                        warn!(query_name, attempt, %status, "retryable HTTP error");
                        last_err = Some(resp.error_for_status().unwrap_err());
                        if attempt < self.config.retries {
                            tokio::time::sleep(self.config.retry_delay).await;
                        }
                        continue;
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

                    if let Some(errors) = body.get("errors").and_then(|e| e.as_array()) {
                        let entries: Vec<GraphqlErrorEntry> = errors
                            .iter()
                            .map(|e| GraphqlErrorEntry {
                                message: e
                                    .get("message")
                                    .and_then(|m| m.as_str())
                                    .unwrap_or("unknown error")
                                    .to_string(),
                            })
                            .collect();
                        let messages = entries
                            .iter()
                            .map(|e| e.message.as_str())
                            .collect::<Vec<_>>()
                            .join("; ");
                        return Err(Error::Graphql {
                            query_name: query_name.to_string(),
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

    /// Query blocks by height/date range and chain status, with full
    /// transaction detail.
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
