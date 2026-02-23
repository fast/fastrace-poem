#![doc = include_str!("../README.md")]

use std::sync::Arc;

use fastrace::prelude::*;
use opentelemetry_semantic_conventions::trace::HTTP_REQUEST_METHOD;
use opentelemetry_semantic_conventions::trace::HTTP_RESPONSE_STATUS_CODE;
use opentelemetry_semantic_conventions::trace::HTTP_ROUTE;
use opentelemetry_semantic_conventions::trace::URL_PATH;
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

type SpanContextExtractor = Arc<dyn Fn(&Request) -> Option<SpanContext> + Send + Sync + 'static>;

/// Middleware for integrating fastrace distributed tracing with Poem web framework.
///
/// This middleware extracts trace context from incoming HTTP requests and creates
/// a new root span for each request, properly linking it to any parent context
/// that might exist from upstream services. By default, the middleware uses the
/// `traceparent` header to extract a span context and falls back to a random context
/// when the header is missing or invalid. If the configured extractor returns `None`,
/// a noop span is used.
///
/// # Example
///
/// ```
/// use fastrace_poem::FastraceMiddleware;
/// use poem::EndpointExt;
/// use poem::Route;
/// use poem::get;
/// use poem::handler;
///
/// let app = Route::new()
///     .at("/ping", get(ping))
///     .with(FastraceMiddleware::default());
/// ```
#[derive(Clone)]
pub struct FastraceMiddleware {
    span_context_extractor: SpanContextExtractor,
}

impl Default for FastraceMiddleware {
    fn default() -> Self {
        Self {
            span_context_extractor: Arc::new(|req| {
                req.headers()
                    .get(TRACEPARENT_HEADER)
                    .and_then(|traceparent| {
                        SpanContext::decode_w3c_traceparent(traceparent.to_str().ok()?)
                    })
                    .or_else(|| Some(SpanContext::random()))
            }),
        }
    }
}

impl FastraceMiddleware {
    /// Configure a custom span context extractor.
    ///
    /// Return `None` to keep the span as noop.
    pub fn with_span_context_extractor<F>(mut self, f: F) -> Self
    where F: Fn(&Request) -> Option<SpanContext> + Send + Sync + 'static {
        self.span_context_extractor = Arc::new(f);
        self
    }
}

impl<E: Endpoint> Middleware<E> for FastraceMiddleware {
    type Output = FastraceEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        FastraceEndpoint {
            inner: ep,
            span_context_extractor: self.span_context_extractor.clone(),
        }
    }
}

/// An endpoint wrapper created by [`FastraceMiddleware`].
///
/// This type is created by the `FastraceMiddleware` and handles the extraction
/// of trace context from requests and the creation of spans around request handlers.
pub struct FastraceEndpoint<E> {
    inner: E,
    span_context_extractor: SpanContextExtractor,
}

impl<E: Endpoint> Endpoint for FastraceEndpoint<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let parent = (self.span_context_extractor)(&req);

        let span = if let Some(parent) = parent {
            // TODO: use low cardinality route once poem supports it.
            let name = format!("{} {}", req.method().as_str(), req.uri().path());

            let root = Span::root(name, parent);

            root.add_properties(|| {
                [
                    (HTTP_REQUEST_METHOD, req.method().to_string()),
                    (URL_PATH, req.uri().path().to_string()),
                    // TODO: use low cardinality route once poem supports it.
                    (HTTP_ROUTE, req.uri().path().to_string()),
                ]
            });

            root
        } else {
            Span::noop()
        };

        async {
            let resp = self.inner.call(req).await?.into_response();
            LocalSpan::add_property(|| {
                (
                    HTTP_RESPONSE_STATUS_CODE,
                    resp.status().as_u16().to_string(),
                )
            });
            Ok(resp)
        }
        .in_span(span)
        .await
    }
}
