//! Real Dioxus-web hook and event callback compilation without Dioxus's global
//! logger feature. The local logger is initialized once per mounted component.
use dioxus::prelude::*;

#[component]
pub fn App() -> Element {
    #[cfg(target_arch = "wasm32")]
    let logger = use_hook(|| std::rc::Rc::new(
        ores_otel_web::browser::BrowserLogger::new("dioxus-probe", |_| Ok(())).unwrap()
    ));
    rsx! {
        button {
            onclick: move |_| {
                #[cfg(target_arch = "wasm32")]
                let _ = logger.info("dioxus.click", None);
            },
            "Telemetry probe"
        }
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub fn launch() { dioxus::launch(App); }
