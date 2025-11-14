#[cfg(feature = "hydrate")]
pub fn main() {
    // no-op
}

#[cfg(not(feature = "hydrate"))]
pub fn main() {
    use leptos::*;
    use leptos_app::App;
    
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    
    mount_to_body(|| {
        view! { <App/> }
    });
    
    #[cfg(not(target_arch = "wasm32"))]
    leptos_app::generate_html();
}
