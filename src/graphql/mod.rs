use anyhow::Result;
use warp::{Filter, Rejection, Reply};
use async_graphql::{EmptySubscription, Schema, Object, SimpleObject};
use async_graphql_warp::{GraphQLResponse, GraphQLBadRequest};
use std::sync::Arc;
use tokio::sync::Mutex;

// Define your GraphQL schema types
#[derive(SimpleObject)]
pub struct User {
    id: String,
    name: String,
    email: String,
}

pub struct Query;

#[Object]
impl Query {
    async fn hello(&self) -> String {
        "Hello, GraphQL!".to_string()
    }

    async fn users(&self) -> Vec<User> {
        vec![
            User { id: "1".to_string(), name: "Alice".to_string(), email: "alice@example.com".to_string() },
            User { id: "2".to_string(), name: "Bob".to_string(), email: "bob@example.com".to_string() },
        ]
    }
}

pub struct Mutation;

#[Object]
impl Mutation {
    async fn add_user(&self, name: String, email: String) -> User {
        // In a real app, you'd save this to a database
        log::info!("Adding new user: {} ({})", name, email);
        User {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            email,
        }
    }
}

pub type AppSchema = Schema<Query, Mutation, EmptySubscription>;

pub fn build_schema() -> AppSchema {
    Schema::build(Query, Mutation, EmptySubscription).finish()
}

pub async fn run_graphql_server() {
    let schema = build_schema();

    let graphql_post = async_graphql_warp::graphql(schema)
        .and_then(|(schema, request): (AppSchema, async_graphql::Request)| async move {
            Ok::<_, Rejection>(GraphQLResponse::from(schema.execute(request).await))
        });

    let routes = warp::path("graphql")
        .and(warp::post().and(graphql_post))
        .or(warp::path("graphql").and(warp::get()).and(graphql_post)); // Allow GET for GraphiQL

    log::info!("Starting GraphQL server on 127.0.0.1:8000/graphql");
    warp::serve(routes).run(([127, 0, 0, 1], 8000)).await;
}

pub fn init() {
    log::info!("GraphQL module initialized.");
}
