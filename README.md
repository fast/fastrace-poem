# fastrace-poem

[![Crates.io](https://img.shields.io/crates/v/fastrace-poem.svg?style=flat-square&logo=rust)](https://crates.io/crates/fastrace-poem)
[![Documentation](https://img.shields.io/docsrs/fastrace-poem?style=flat-square&logo=rust)](https://docs.rs/fastrace-poem/)
[![MSRV 1.83.0](https://img.shields.io/badge/MSRV-1.83.0-green?style=flat-square&logo=rust)](https://www.whatrustisit.com)
[![CI Status](https://img.shields.io/github/actions/workflow/status/fast/fastrace-poem/ci.yml?style=flat-square&logo=github)](https://github.com/fast/fastrace-poem/actions)
[![License](https://img.shields.io/crates/l/fastrace-poem?style=flat-square)](https://github.com/fast/fastrace-poem/blob/main/LICENSE)

Distributed tracing integration for [Poem](https://github.com/poem-web/poem) web framework with [fastrace](https://crates.io/crates/fastrace).

## Overview

`fastrace-poem` provides middleware for the Poem web framework to enable distributed tracing with automatic context propagation. This helps you track requests as they flow through your microservice architecture, giving you valuable insights for debugging, performance analysis, and system understanding.

### What is Context Propagation?

Distributed tracing works by passing trace context (trace IDs, span IDs, etc.) between services. This allows individual service traces to be connected into a complete picture of a request's journey through your system.

Context propagation refers to the act of passing this trace context between services. In HTTP-based systems, this is typically done via HTTP headers like `traceparent` following the [W3C Trace Context](https://www.w3.org/TR/trace-context/) specification.

## Features

- 🔄 **Automatic context propagation** via W3C traceparent headers.
- 🌉 **Seamless integration** with Poem's middleware system.
- 🔗 **Request tracing** with proper parent-child span relationships.
- 📊 **Full compatibility** with fastrace's collection and reporting capabilities.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
fastrace = "0.7"
fastrace-poem = "0.1"
```

## Usage

### Server Integration

```rust
use fastrace::collector::{Config, ConsoleReporter};
use fastrace_poem::FastraceMiddleware;
use poem::{get, handler, EndpointExt, Request, Response, Route, Server};
use poem::listener::TcpListener;

#[handler]
#[fastrace::trace] // Trace individual handlers.
fn ping() -> Response {
    Response::builder().body("pong")
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Configure fastrace reporter.
    fastrace::set_reporter(ConsoleReporter, Config::default());
    
    // Add the FastraceMiddleware to your routes.
    let app = Route::new()
        .at("/ping", get(ping))
        .with(FastraceMiddleware);
    
    Server::new(TcpListener::bind("0.0.0.0:8080"))
        .run(app)
        .await?;
    
    fastrace::flush();
    Ok(())
}
```

### Client Usage with fastrace-reqwest

To propagate trace context from clients to your Poem service:

```rust
use fastrace::prelude::*;
use fastrace_reqwest::traceparent_headers;
use reqwest::Client;

#[fastrace::trace]
async fn send_request() {
    let client = Client::new();
    let response = client
        .get("http://your-poem-service/endpoint")
        .headers(traceparent_headers()) // Adds traceparent header.
        .send()
        .await
        .unwrap();
    
    // Process response...
}
```

## How It Works

1. When a request arrives, the middleware checks for a `traceparent` header.
2. If present, it extracts the trace context; otherwise, it creates a new random context.
3. A new root span is created for the request using the URI as the name.
4. The request handler is executed within this span, and any child spans are properly linked.
5. The trace is then collected by your configured fastrace reporter.

### Complete Example

Check out the [examples directory]([./examples](https://github.com/fast/fastrace-poem/tree/main/examples)) for complete working examples showing:

- `client.rs` - How to send requests with trace context
- `server.rs` - How to receive and process trace context using `fastrace-poem`

To run the examples:

```bash
# First start the server
cargo run --example server

# Then in another terminal, run the client
cargo run --example client
```

## License

This project is licensed under the [Apache-2.0](./LICENSE) license.
