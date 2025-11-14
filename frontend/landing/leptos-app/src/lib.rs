pub mod app;
pub use app::*;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use leptos::*;
    use wasm_bindgen::prelude::*;
    
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    
    mount_to_body(|| view! { <App/> });
}

#[cfg(not(feature = "hydrate"))]
pub fn generate_html() {
    // For CSR, we only need the static index.html
    // No SSR rendering needed
} 