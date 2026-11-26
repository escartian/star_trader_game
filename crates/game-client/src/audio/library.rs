//! Browser music library: the blocks are not built into the game, they are
//! fetched from `music/<file>` beside the page after it starts, one at a
//! time in `music_graph::load_order` (the start block and its slow neighbours
//! first, so the main menu's music can begin while the rest arrives). Each
//! file is kept for the rest of the visit.
use std::sync::Mutex;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

/// Loaded blocks by `music_graph::NAMES` index.
static LOADED: Mutex<Vec<Option<&'static [u8]>>> = Mutex::new(Vec::new());

/// A block's bytes, once fetched.
pub fn get(index: usize) -> Option<&'static [u8]> {
    LOADED.lock().ok()?.get(index).copied().flatten()
}

fn store(index: usize, bytes: Vec<u8>) {
    if let Ok(mut loaded) = LOADED.lock() {
        if loaded.len() <= index {
            loaded.resize(index + 1, None);
        }
        // Kept for the whole visit: decoders borrow it for 'static.
        loaded[index] = Some(Box::leak(bytes.into_boxed_slice()));
    }
}

/// Starts fetching every block in the background, in play order.
pub fn start() {
    let order = match super::music_graph::load_order() {
        Ok(order) => order,
        Err(error) => {
            log(&format!("Music manifest: {error}"));
            return;
        }
    };
    wasm_bindgen_futures::spawn_local(async move {
        for index in order {
            let Some(file) = super::music_graph::file_name(index) else {
                continue;
            };
            match fetch(&format!("music/{file}")).await {
                Ok(bytes) => store(index, bytes),
                Err(error) => log(&format!("Music {file} could not be loaded: {error}")),
            }
        }
    });
}

async fn fetch(url: &str) -> Result<Vec<u8>, String> {
    let window = web_sys::window().ok_or("no window")?;
    let response: web_sys::Response = JsFuture::from(window.fetch_with_str(url))
        .await
        .map_err(describe)?
        .dyn_into()
        .map_err(describe)?;
    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }
    let buffer = JsFuture::from(response.array_buffer().map_err(describe)?)
        .await
        .map_err(describe)?;
    Ok(js_sys::Uint8Array::new(&buffer).to_vec())
}

fn describe(value: wasm_bindgen::JsValue) -> String {
    value.as_string().unwrap_or_else(|| format!("{value:?}"))
}

fn log(message: &str) {
    web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(message));
}
