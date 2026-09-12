#![cfg(all(feature = "browser", target_arch = "wasm32"))]
use ores_otel_web::browser::{BrowserLogger, child_trace, new_trace, traced_request};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen_test::*;
use web_sys::{RequestCredentials, RequestInit, RequestMode, RequestRedirect};
wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn two_island_loggers_do_not_share_context_or_ids() {
    let records = Rc::new(RefCell::new(Vec::new()));
    let sink = records.clone();
    let a = BrowserLogger::new("leptos-island", move |record| {
        sink.borrow_mut().push(record.clone());
        Ok(())
    })
    .unwrap();
    let sink = records.clone();
    let b = BrowserLogger::new("dioxus-island", move |record| {
        sink.borrow_mut().push(record.clone());
        Ok(())
    })
    .unwrap();
    let parent = new_trace(false).unwrap();
    let child = child_trace(&parent).unwrap();
    let first = a.info("action", Some(&parent)).unwrap();
    let second = b.info("action", Some(&child)).unwrap();
    let unscoped = a.info("local", None).unwrap();
    assert_ne!(first.id, second.id);
    assert_eq!(first.trace_id, second.trace_id);
    assert_ne!(first.fields["otel.span_id"], second.fields["otel.span_id"]);
    assert!(unscoped.trace_id.is_none());
    assert!(!first.timestamp.starts_with("1970"));
    assert_eq!(records.borrow().len(), 3);
}

#[wasm_bindgen_test]
fn requests_keep_method_and_application_body_but_cannot_leak_off_origin() {
    let trace = new_trace(true).unwrap();
    let init = RequestInit::new();
    init.set_method("POST");
    init.set_body(&wasm_bindgen::JsValue::from_str("test-body"));
    let request = traced_request("/api/probe", &init, &trace).unwrap();
    assert_eq!(request.method(), "POST");
    assert_eq!(request.mode(), RequestMode::SameOrigin);
    assert_eq!(request.credentials(), RequestCredentials::SameOrigin);
    assert_eq!(request.redirect(), RequestRedirect::Error);
    assert_eq!(
        request.headers().get("traceparent").unwrap().unwrap(),
        trace.to_string()
    );
    assert!(traced_request("https://example.invalid/", &init, &trace).is_err());
    assert!(traced_request("//example.invalid/", &init, &trace).is_err());
}

#[wasm_bindgen_test]
fn sampling_flags_and_child_span_identity_survive_framework_handoff() {
    let parent = new_trace(true).unwrap();
    let child = child_trace(&parent).unwrap();

    assert!(parent.sampled());
    assert!(child.sampled());
    assert_eq!(child.trace_id(), parent.trace_id());
    assert_ne!(child.span_id(), parent.span_id());
    assert_eq!(child.flags(), parent.flags());
}

#[wasm_bindgen_test]
fn unsampled_context_stays_unsampled_across_child_creation() {
    let parent = new_trace(false).unwrap();
    let child = child_trace(&parent).unwrap();

    assert!(!parent.sampled());
    assert!(!child.sampled());
    assert_eq!(child.trace_id(), parent.trace_id());
    assert_ne!(child.span_id(), parent.span_id());
}

#[wasm_bindgen_test]
fn oversized_service_identity_and_sink_failure_are_explicit_errors() {
    let oversized = "x".repeat(129);
    assert!(BrowserLogger::new(&oversized, |_| Ok(())).is_err());

    let logger = BrowserLogger::new("dioxus-web", |_| Err("export unavailable".into())).unwrap();
    assert_eq!(
        logger.info("event", None).unwrap_err(),
        "export unavailable"
    );
}

#[wasm_bindgen_test]
fn non_http_and_credential_bearing_requests_fail_closed() {
    let trace = new_trace(false).unwrap();
    let init = RequestInit::new();

    for url in [
        "data:text/plain,hello",
        "javascript:alert(1)",
        "https://user:password@example.invalid/api",
    ] {
        assert!(traced_request(url, &init, &trace).is_err(), "accepted {url}");
    }
}
