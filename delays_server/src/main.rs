//! A simple HTTP server that introduces artificial delays in responses
//!
//! This server provides an endpoint that accepts a delay duration(milliseconds) and message,
//! waits for the specified time, then returns the message. It's useful for testing
//! and simulating slow network conditions.

use axum::{
    Router,
    extract::{Path, State},
    routing::get,
};
use std::{
    env,
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::time::sleep;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
struct AppState {
    counter: Arc<AtomicUsize>,
}

async fn delay_handler(
    Path((delay_ms, message)): Path<(u64, String)>,
    State(state): State<AppState>,
) -> String {
    let count = state.counter.fetch_add(1, Ordering::SeqCst);
    println!("#{count} - {delay_ms}ms: {message}");
    sleep(Duration::from_millis(delay_ms)).await;
    message
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let host = env::args()
        .nth(1)
        .unwrap_or_else(|| String::from("127.0.0.1"));

    let addr = format!("{}:8080", host)
        .parse::<SocketAddr>()
        .unwrap_or_else(|_| {
            // If host is a hostname rather than IP, default to localhost
            SocketAddr::from(([127, 0, 0, 1], 8080))
        });

    let state = AppState {
        counter: Arc::new(AtomicUsize::new(1)),
    };

    let app = Router::new()
        .route("/:delay/:message", get(delay_handler))
        .with_state(state);

    println!("Server starting on http://{}", addr);

    // Starting the server with tokio
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
