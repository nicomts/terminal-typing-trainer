mod anim;
mod app;
mod corpus;
mod event;
#[cfg(not(target_arch = "wasm32"))]
mod native;
mod session;
mod stats;
mod storage;
mod theme;
mod ui;
#[cfg(target_arch = "wasm32")]
mod web;

// The same code builds two apps. `cargo run` builds the terminal app;
// `trunk serve` builds for `wasm32` (WebAssembly) and runs it in a browser.

#[cfg(not(target_arch = "wasm32"))]
fn main() -> color_eyre::Result<()> {
    native::main()
}

#[cfg(target_arch = "wasm32")]
fn main() -> color_eyre::Result<()> {
    web::main()
}
