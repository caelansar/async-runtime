# Cmoon: A Custom Async Runtime for Rust

Cmoon is a lightweight, educational async runtime implementation for Rust. This project demonstrates the core concepts of asynchronous programming in Rust by implementing a custom executor, reactor, and I/O utilities from scratch.

## Project Structure

The project is organized as a Cargo workspace with multiple crates:

- **cmoon** (root crate): The core runtime implementation
- **delays_server**: A test HTTP server
- **rt_macro**: Procedural macros for ergonomic async function execution

## Features

- **Custom Executor**: A single-threaded executor that schedules and executes async tasks
- **Event-driven Reactor**: A non-blocking I/O implementation using `mio` for efficient I/O operations
- **Procedural Macros**: Macros to simplify using async functions with `#[cmoon::main]` and `#[cmoon::test]` attributes
- **Delay Server**: A sample HTTP server that introduces artificial delays in responses

## Getting Started

### Prerequisites

- Rust (edition 2024)
- Cargo

### Building the Project

```bash
# Build the entire workspace
cargo build

# Build a specific package
cargo build -p delays_server
```

### Running the Delay Server

The delay server provides an HTTP endpoint that introduces artificial delays in responses:

```bash
# Run with default settings (localhost:8080)
cargo run -p delays_server
```

Once running, you can access it at:
```
http://localhost:8080/{delay_ms}/{message}
```

Where:
- `{delay_ms}` is the delay in milliseconds
- `{message}` is the message to echo back after the delay

### Running Tests

```bash
# Run all tests
cargo test

# Run tests for a specific package
cargo test -p cmoon
```

## Usage Examples

### Using the Core Runtime

```rust
use cmoon::block_on;

fn main() {
    block_on(async {
        // Your async code here
        let result = perform_async_operation().await;
        println!("Result: {}", result);
    });
}

async fn perform_async_operation() -> String {
    // Example async operation
    cmoon::io::http::Http::get("http://localhost:8080/100/hello").await
}
```

### Using the Procedural Macros

Enable the `macro` feature to use the procedural macros:

```rust
use cmoon;

#[cmoon::main]
async fn main() {
    // Your async code here
    let result = cmoon::io::http::Http::get("http://localhost:8080/100/hello").await;
    println!("Result: {}", result);
}
```

## Implementation Details

### Executor

The executor is responsible for scheduling and executing async tasks. It manages:
- Task scheduling and waking
- Task execution through polling
- Efficient thread parking when no tasks are ready to progress

### Reactor

The reactor handles I/O events using `mio` to provide non-blocking I/O capabilities:
- Registering I/O resources for event notifications
- Waking tasks when I/O operations can make progress
- Managing wakers for suspended futures

### HTTP Implementation

The `Http` module demonstrates how to implement async I/O with the custom runtime:
- Non-blocking TCP connections
- Future-based API for HTTP requests
- Integration with the reactor for efficient I/O polling

## License

MIT License

## Acknowledgments

This project serves as a practical example of custom runtime implementation. 