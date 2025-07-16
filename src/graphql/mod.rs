use async_graphql::{EmptySubscription, Schema};
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql_warp::{GraphQLBadRequest, GraphQLResponse};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use warp::{http::Response as HttpResponse, Filter, Rejection};

/// A simple GraphQL client for interacting with a GraphQL API.
/// This module provides basic functionality to send GraphQL queries and mutations.
pub struct GraphQLClient {
    endpoint: String,
    http_client: reqwest::Client,
    headers: HashMap<String, String>,
}

impl GraphQLClient {
    /// Creates a new `GraphQLClient` instance.
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            http_client: reqwest::Client::new(),
            headers: HashMap::new(),
        }
    }

    /// Adds a header to all outgoing GraphQL requests.
    pub fn add_header(&mut self, name: &str, value: &str) {
        self.headers.insert(name.to_string(), value.to_string());
    }

    /// Sends a GraphQL query to the configured endpoint.
    pub async fn query(&self, query: &str, variables: Option<serde_json::Value>) -> Result<serde_json::Value, String> {
        let mut request_body = serde_json::json!({
            "query": query,
        });
        if let Some(vars) = variables {
            request_body["variables"] = vars;
        }

        let mut request = self.http_client.post(&self.endpoint)
            .json(&request_body);

        for (key, value) in &self.headers {
            request = request.header(key, value);
        }

        let response = request.send().await.map_err(|e| format!("Failed to send GraphQL request: {}", e))?;
        let response_json: serde_json::Value = response.json().await.map_err(|e| format!("Failed to parse GraphQL response: {}", e))?;

        if let Some(errors) = response_json.get("errors") {
            Err(format!("GraphQL errors: {}", errors))
        } else {
            Ok(response_json["data"].clone())
        }
    }

    /// Sends a GraphQL mutation to the configured endpoint.
    pub async fn mutate(&self, mutation: &str, variables: Option<serde_json::Value>) -> Result<serde_json::Value, String> {
        // Mutations are handled similarly to queries, just with a different operation type.
        self.query(mutation, variables).await
    }
}

// --- Example GraphQL Server (for testing/demonstration purposes) ---

// Define a simple GraphQL schema
pub struct Query;

#[async_graphql::Object]
impl Query {
    async fn hello(&self) -> String {
        "Hello from NeoTerm GraphQL!".to_string()
    }

    async fn add_numbers(&self, a: i32, b: i32) -> i32 {
        a + b
    }
}

pub struct Mutation;

#[async_graphql::Object]
impl Mutation {
    async fn echo(&self, message: String) -> String {
        format!("Echo: {}", message)
    }
}

pub type AppSchema = Schema<Query, Mutation, EmptySubscription>;

/// Creates a GraphQL server filter for Warp.
pub fn create_graphql_server() -> impl Filter<Extract = (GraphQLResponse,), Error = Rejection> + Clone {
    let schema = Schema::build(Query, Mutation, EmptySubscription).finish();

    async_graphql_warp::graphql(schema).and_then(
        |(schema, request): (AppSchema, async_graphql::Request)| async move {
            Ok::<_, Rejection>(async_graphql_warp::Response::from(schema.execute(request).await))
        },
    )
}

/// Creates a GraphQL Playground filter for Warp.
pub fn create_graphql_playground() -> impl Filter<Extract = (HttpResponse<String>,), Error = Rejection> + Clone {
    warp::path("playground").map(|| {
        HttpResponse::builder()
            .header("content-type", "text/html")
            .body(playground_source(GraphQLPlaygroundConfig::new("/graphql")))
            .unwrap()
    })
}

pub fn init() {
    println!("graphql module initialized: Provides GraphQL client and server capabilities.");
}
