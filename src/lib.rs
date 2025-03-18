#![doc = include_str!("../README.md")]

use fastrace::prelude::*;
use poem::Endpoint;
use poem::IntoResponse;
use poem::Middleware;
use poem::Request;
use poem::Response;
use poem::Result;

/// The standard [W3C Trace Context](https://www.w3.org/TR/trace-context/) header name for passing trace information.
///
/// This is the header key used to propagate trace context between services according to
/// the W3C Trace Context specification.
pub const TRACEPARENT_HEADER: &str = "traceparent";

/// Middleware for integrating fastrace distributed tracing with Poem web framework.
///
/// This middleware extracts trace context from incoming HTTP requests and creates
/// a new root span for each request, properly linking it to any parent context
/// that might exist from upstream services.
///
/// # Example
///
/// ```
/// use fastrace_poem::FastraceMiddleware;
/// use poem::Route;
/// use poem::get;
/// use poem::handler;
///
/// let app = Route::new().at("/ping", get(ping)).with(FastraceMiddleware);
/// ```
#[derive(Default)]
pub struct FastraceMiddleware;

impl<E: Endpoint> Middleware<E> for FastraceMiddleware {
    type Output = FastraceEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        FastraceEndpoint { inner: ep }
    }
}

/// An endpoint wrapper created by [`FastraceMiddleware`].
///
/// This type is created by the `FastraceMiddleware` and handles the extraction
/// of trace context from requests and the creation of spans around request handlers.
pub struct FastraceEndpoint<E> {
    inner: E,
}

impl<E: Endpoint> Endpoint for FastraceEndpoint<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let headers = req.headers();
        let parent = headers
            .get(TRACEPARENT_HEADER)
            .and_then(|traceparent| SpanContext::decode_w3c_traceparent(traceparent.to_str().ok()?))
            .unwrap_or(SpanContext::random());
        let root = Span::root(req.uri().to_string(), parent);
        self.inner
            .call(req)
            .in_span(root)
            .await
            .map(|resp| resp.into_response())
    }
}
