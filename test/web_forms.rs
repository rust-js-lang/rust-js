//! Every JS form the webapi crate's bindings use (ADR 0024), for the test to
//! check in the generated JS.

use js::uint8_array;
use webapi::{Event, document, element, event, event_target, html_input_element, node, text_decoder, text_encoder, window};

pub fn forms() -> String {
    let input = html_input_element::unchecked_from(document::create_element_named(document, "input"));
    html_input_element::set_value(input, "typed");
    let app = document::get_element_by_id(document, "app").expect("the page has an #app");
    element::append(app, input);
    element::append(app, "!");
    let ping: &Event = event::new("ping");
    event_target::add_event_listener_named(app, "ping", Box::new(|e| event::prevent_default(e)));
    let _ = event_target::dispatch_event(window, ping);
    node::text_content(app).unwrap() + &html_input_element::value(input)
}

/// Optional arguments give more forms: `encode_with_input(e, text)` is
/// `e.encode(text)`, and a union takes its member as it is (ADR 0229):
/// `decode_with_input(d, bytes)` is `d.decode(bytes)`.
pub fn round_trip(text: &str) -> (u32, String) {
    let bytes = text_encoder::encode_with_input(text_encoder::new(), text);
    let back = text_decoder::decode_with_input(text_decoder::new_with_label("utf-8"), bytes);
    (uint8_array::length(bytes), back)
}

/// A `BodyInit` of a `Blob`, and a `RequestInfo` of a URL: each the value
/// itself, as TypeScript's union takes it.
pub fn bodies(url: &str) -> (&'static webapi::Response, js::Promise<&'static webapi::Response>) {
    let blob = webapi::blob::new();
    let response = webapi::response::new_with_body(blob);
    (response, window::fetch(window, url))
}

/// A `sequence` is a slice (ADR 0219): a `Blob` of its parts, each a
/// `BlobPart`, and the clipboard written a list of items, each `new
/// Blob([text, "!"])` and `navigator.clipboard.write(items)`.
pub fn copied(text: &str, items: &[&webapi::ClipboardItem]) -> (&'static webapi::Blob, js::Promise<()>) {
    let blob = webapi::blob::new_with_blob_parts(&[text.into(), "!".into()]);
    (blob, webapi::clipboard::write(webapi::navigator::clipboard(webapi::navigator), items))
}

/// A frame's window, a message's sender, and the page's clock:
/// `frame.contentWindow`, `e.source` and `window.performance.now()`.
pub fn from_frame(frame: &webapi::HTMLIFrameElement, e: &Event) -> (bool, f64) {
    let sender = webapi::message_event::source(webapi::message_event::unchecked_from(e));
    let window_of = webapi::html_i_frame_element::content_window(frame);
    (js::object::is(&sender, &window_of), webapi::performance::now(window::performance(window)))
}

/// A canvas, which a library like canvas-confetti draws on: its size, set
/// and read, `canvas.width = 320`.
pub fn canvas_size() -> (u32, u32) {
    let canvas = webapi::html_canvas_element::unchecked_from(document::create_element_named(document, "canvas"));
    webapi::html_canvas_element::set_width(canvas, 320);
    webapi::html_canvas_element::set_height(canvas, 200);
    (webapi::html_canvas_element::width(canvas), webapi::html_canvas_element::height(canvas))
}

/// A static method, the class's own, as react.dev's DownloadButton asks
/// whether a script may be an import map and makes a URL of a blob:
/// `HTMLScriptElement.supports("importmap")`, `URL.createObjectURL(blob)`.
pub fn statics(blob: &webapi::Blob) -> (bool, String) {
    let url = webapi::url::create_object_url(blob);
    webapi::url::revoke_object_url(&url);
    (webapi::html_script_element::supports("importmap"), url)
}

/// What a function gives, of each WebIDL type: a sequence is a `Vec`, a
/// frozen array a slice, a `long long` an `f64` (a JS number), a `float` an
/// `f32`, a dictionary a struct of `Option`s, and a constant a `const`.
pub fn kinds(
    el: &webapi::Element,
    blob: &webapi::Blob,
    param: &webapi::AudioParam,
    bytes: &js::Uint8Array,
) -> (Vec<String>, usize, f64, f32, u16, Option<f64>) {
    let names = element::get_attribute_names(el);
    let languages = webapi::navigator::languages(webapi::navigator);
    let into = text_encoder::encode_into(text_encoder::new(), "hi", bytes);
    (names, languages.len(), webapi::blob::size(blob), webapi::audio_param::value(param), node::ELEMENT_NODE, into.written)
}

/// An event handler property, `onclick`: a closure given the event its name
/// is on the target, a button's click a `PointerEvent`, or `None`, `null`.
pub fn handlers(button: &webapi::HTMLButtonElement) -> bool {
    webapi::html_element::set_onclick(
        button,
        Some(Box::new(|e: &webapi::PointerEvent| {
            event::prevent_default(e);
        })),
    );
    let set = webapi::html_element::onclick(button).is_some();
    webapi::html_element::set_onclick(button, None);
    set
}

/// A constructor of a class from a spec the crate didn't read before.
pub fn socket(url: &str) -> &'static webapi::WebSocket {
    webapi::web_socket::new(url)
}
