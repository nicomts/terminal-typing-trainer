//! The browser app: the same `App` and `ui::render`, drawn into the web
//! page by Ratzilla instead of into a terminal.
//!
//! A browser has no loop we can own. Instead it calls us back: once per
//! key press, and once per animation frame (about 60 times a second).

use std::cell::RefCell;
use std::rc::Rc;

use color_eyre::Result;
use color_eyre::eyre::eyre;
use ratatui::Terminal;
use ratzilla::backend::webgl2::WebGl2BackendOptions;
use ratzilla::{FontAtlasConfig, WebGl2Backend, WebRenderer};
use web_sys::KeyboardEvent;
use web_sys::wasm_bindgen::closure::Closure;
use web_sys::wasm_bindgen::{JsCast, JsValue};
use web_time::Instant;

use crate::app::App;
use crate::event::{Event, Key};
use crate::{corpus, storage, theme, ui};

/// The bundled font, registered in `index.html`.
const BUNDLED_FONT: &str = "JetBrains Mono Bold";
/// Fonts to draw with, first one found wins: the bundled font, then system
/// monospace fonts in case it failed to load. All have the box-drawing,
/// block and arrow characters the UI uses.
const FONTS: [&str; 5] = [
    BUNDLED_FONT,
    "ui-monospace",
    "Menlo",
    "Consolas",
    "DejaVu Sans Mono",
];
/// In CSS pixels.
const FONT_SIZE: f32 = 16.0;

/// Loads the bundled font, then starts the app.
///
/// The glyphs are drawn into a texture once, when the WebGL backend is
/// created, so the font must be loaded before that or the texture would
/// use a fallback font. Loading takes time, and the browser can't wait
/// inside `main`: `load` returns a JavaScript Promise, and `then` calls
/// our closure once the font is ready.
pub fn main() -> Result<()> {
    let fonts = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| eyre!("no web page to draw in"))?
        .fonts();
    let promise = fonts.load(&format!("{FONT_SIZE}px \"{BUNDLED_FONT}\""));

    let on_loaded = Closure::<dyn FnMut(JsValue)>::new(|_| start_or_show_error());
    // The same closure handles success and failure: if the font can't be
    // loaded, the app still starts, drawn with a system font.
    let _ = promise.then2(&on_loaded, &on_loaded);
    // The closure must outlive `main`, so `forget` keeps it alive for good.
    // It is created once, so this costs a few bytes.
    on_loaded.forget();
    Ok(())
}

/// Starts the app. If that fails, the error is shown in the page, since
/// a browser has no terminal to print it to.
fn start_or_show_error() {
    if let Err(error) = start() {
        show_error(&format!("Could not start: {error}"));
    }
}

fn start() -> Result<()> {
    let commands = corpus::load()?;
    let profile = storage::load_browser()?;
    let app = App::new(commands, profile, rand::make_rng(), Instant::now())?;

    // Both callbacks below need the same `App`, and the key callback
    // changes it. Each callback must own what it uses (`'static`), so a
    // plain `&mut App` can't be shared between them. `Rc` lets both own
    // it, and `RefCell` allows changing it through that shared ownership,
    // checking at run time that only one callback uses it at a time.
    // The browser runs everything on one thread, so `Arc`/`Mutex` aren't needed.
    let app = Rc::new(RefCell::new(app));

    listen_for_keys(Rc::clone(&app))?;

    // WebGL draws each cell from a font texture sized from the font's own
    // `█`, so block characters (the logo, the sparkline) fill their cells
    // exactly. The DOM backend leaves thin gaps between rows instead.
    let font = FontAtlasConfig::dynamic(&FONTS, FONT_SIZE);
    let mut options = WebGl2BackendOptions::new().font_atlas_config(font);
    // The window is rarely a whole number of cells; the leftover pixels at
    // the right and bottom edges get this color.
    if let Some(background) = theme::app().bg {
        options = options.canvas_padding_color(background);
    }
    let terminal = Terminal::new(WebGl2Backend::new_with_options(options)?)?;
    terminal.draw_web(move |frame| {
        // The tick moves the animations forward, like the terminal loop
        // does on each timeout. `update` still runs before `render`.
        app.borrow_mut().update(Event::Tick { at: Instant::now() });
        ui::render(frame, &app.borrow());
    });
    Ok(())
}

/// Sends key presses anywhere on the page to `app`.
///
/// Ratzilla's own `on_key_event` only hears keys while its grid has focus,
/// and can't stop the browser's default action (Tab moving focus, `'`
/// opening Firefox's quick find), so we listen on the whole document.
fn listen_for_keys(app: Rc<RefCell<App>>) -> Result<()> {
    let on_key = Closure::<dyn FnMut(KeyboardEvent)>::new(move |event: KeyboardEvent| {
        let Some(key) = to_key(&event) else {
            return;
        };
        event.prevent_default();

        let mut app = app.borrow_mut();
        app.update(Event::Key {
            key,
            at: Instant::now(),
        });
        // Same as the terminal loop: `update` only flags it, we save here.
        if app.profile_changed {
            if let Err(error) = storage::save_browser(&app.profile) {
                web_sys::console::error_1(&error.to_string().into());
            }
            app.profile_changed = false;
        }
    });

    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| eyre!("no web page to listen on"))?;
    document
        .add_event_listener_with_callback("keydown", on_key.as_ref().unchecked_ref())
        .map_err(|error| eyre!("could not listen for keys: {error:?}"))?;
    // The page keeps calling the listener for as long as it is open, so the
    // closure must never be freed. `forget` hands it over to the browser.
    on_key.forget();
    Ok(())
}

/// The `Key` for a browser key event, or `None` if the app doesn't use it.
///
/// Cmd and Ctrl shortcuts are left to the browser (Cmd+R reloads, Ctrl+C
/// copies), like the terminal app ignores them. Alt is allowed, because on
/// many keyboards symbols need it: Option+1 types `|` on a Spanish Mac.
/// On Windows, AltGr also reports Ctrl, so it is checked separately.
fn to_key(event: &KeyboardEvent) -> Option<Key> {
    let alt_gr = event.get_modifier_state("AltGraph");
    if event.meta_key() || (event.ctrl_key() && !alt_gr) {
        return None;
    }
    // `key()` is the text the key types ("a", "|") or its name ("Enter").
    let name = event.key();
    let mut chars = name.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        return Some(Key::Char(c));
    }
    match name.as_str() {
        "Enter" => Some(Key::Enter),
        "Escape" => Some(Key::Esc),
        "Tab" => Some(Key::Tab),
        "Backspace" => Some(Key::Backspace),
        "ArrowUp" => Some(Key::Up),
        "ArrowDown" => Some(Key::Down),
        _ => None,
    }
}

/// Replaces the page with `message`.
fn show_error(message: &str) {
    let body = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.body());
    if let Some(body) = body {
        body.set_text_content(Some(message));
    }
}
