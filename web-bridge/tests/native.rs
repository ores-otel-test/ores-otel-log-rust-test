#![cfg(all(feature = "native", not(target_arch = "wasm32")))]
use axum::{body::Body, extract::Extension, http::{Request, StatusCode}, routing::get, Router};
use next_loggers::{LogRecord, Logger, LoggerError, Options, Transport};
use ores_otel_web::{server::install_with_logger, TraceParent};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

#[derive(Default)]
struct Capture(Mutex<Vec<LogRecord>>);
impl Transport for Capture {
    fn write(&self, record: &LogRecord) -> Result<(), LoggerError> {
        self.0.lock().unwrap().push(record.clone()); Ok(())
    }
}
fn logger(capture: Arc<Capture>) -> Logger {
    Logger::new(Options { console: false, app_name: "external-consumer".into(), ..Options::default() }.with_transport(capture))
}
fn request(trace: &str) -> Request<Body> {
    Request::builder().uri("/?token=must-not-log").header("traceparent", trace)
        .header("authorization", "Bearer must-not-log").header("cookie", "session=must-not-log")
        .header("baggage", "tenant=must-not-log").body(Body::empty()).unwrap()
}

#[tokio::test]
async fn interleaved_requests_are_isolated_and_sensitive_inputs_stay_out() {
    let capture = Arc::new(Capture::default());
    let app = install_with_logger(Router::new().route("/", get(|Extension(trace): Extension<TraceParent>| async move {
        tokio::task::yield_now().await;
        let actual = next_loggers::current_log_context();
        assert_eq!(actual.trace_id.as_deref(), Some(trace.trace_id()));
        assert_eq!(actual.span_id.as_deref(), Some(trace.span_id()));
        assert!(actual.logged_in_user.is_empty()); assert!(actual.baggage.is_empty());
        StatusCode::ACCEPTED
    })), logger(capture.clone()));
    let a = "00-11111111111111111111111111111111-1111111111111111-00";
    let b = "00-22222222222222222222222222222222-2222222222222222-01";
    let (ra, rb) = tokio::join!(app.clone().oneshot(request(a)), app.oneshot(request(b)));
    for (response, expected) in [(ra.unwrap(), a), (rb.unwrap(), b)] {
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let actual: TraceParent = response.headers()["traceparent"].to_str().unwrap().parse().unwrap();
        let expected: TraceParent = expected.parse().unwrap();
        assert_eq!(actual.trace_id(), expected.trace_id());
        assert_ne!(actual.span_id(), expected.span_id());
        assert_eq!(actual.flags(), expected.flags());
    }
    assert!(next_loggers::current_log_context().trace_id.is_none());
    let records = capture.0.lock().unwrap();
    assert_eq!(records.len(), 2);
    assert_ne!(records[0].trace_id, records[1].trace_id);
    for record in records.iter() {
        let serialized = record.to_json().unwrap();
        assert!(!serialized.contains("must-not-log"));
        assert_eq!(record.schema, "next-loggers/v1");
    }
}

#[tokio::test]
async fn malformed_and_duplicate_context_cannot_be_reflected() {
    for headers in [vec!["bad-context"], vec!["00-11111111111111111111111111111111-1111111111111111-01"; 2]] {
        let mut request = Request::builder().uri("/");
        for value in headers { request = request.header("traceparent", value); }
        let app = install_with_logger(Router::new().route("/", get(|| async { "ok" })), logger(Arc::default()));
        let response = app.oneshot(request.body(Body::empty()).unwrap()).await.unwrap();
        let trace: TraceParent = response.headers()["traceparent"].to_str().unwrap().parse().unwrap();
        assert!(!trace.sampled());
        assert_ne!(trace.trace_id(), "11111111111111111111111111111111");
    }
}

struct FailingSink;
impl Transport for FailingSink {
    fn write(&self, _: &LogRecord) -> Result<(), LoggerError> { Err(LoggerError("offline".into())) }
}
#[tokio::test]
async fn exporter_failure_does_not_replace_application_response() {
    let logger = Logger::new(Options { console: false, ..Options::default() }.with_transport(Arc::new(FailingSink)));
    let app = install_with_logger(Router::new().route("/", get(|| async { StatusCode::IM_A_TEAPOT })), logger);
    let response = app.oneshot(Request::new(Body::empty())).await.unwrap();
    assert_eq!(response.status(), StatusCode::IM_A_TEAPOT);
}
