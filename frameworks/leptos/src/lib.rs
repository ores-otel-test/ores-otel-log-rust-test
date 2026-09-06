//! Compiles the real Leptos event boundary against the browser adapter. SSR
//! does not access Window, crypto, clocks, or install a browser logger.
use leptos::prelude::*;

#[component]
pub fn App() -> impl IntoView {
    #[cfg(target_arch = "wasm32")]
    let logger = ores_otel_web::browser::BrowserLogger::new("leptos-probe", |_| Ok(())).unwrap();
    view! {
        <button on:click=move |_| {
            #[cfg(target_arch = "wasm32")]
            let _ = logger.info("leptos.click", None);
        }>"Telemetry probe"</button>
    }
}

#[cfg(all(feature = "csr", target_arch = "wasm32"))]
pub fn mount() { leptos::mount::mount_to_body(App); }
#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
pub fn hydrate() { leptos::mount::hydrate_body(App); }
