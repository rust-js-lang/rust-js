//! Every JS form the webapi crate's bindings use (ADR 0024), for the test to
//! check in the generated JS.

use js::uint8_array;
use webapi::{Event, document, element, event, event_target, html_input_element, node, text_decoder, text_encoder, window};

pub fn forms() -> String {
    let input = html_input_element::unchecked_from(document::create_element(document, "input"));
    html_input_element::set_value(input, "typed");
    let app = document::get_element_by_id(document, "app").expect("the page has an #app");
    element::append(app, input.into());
    element::append(app, "!".into());
    let ping: &Event = event::new("ping");
    event_target::add_event_listener(app, "ping", Box::new(|e| event::prevent_default(e)));
    let _ = event_target::dispatch_event(window, ping);
    node::text_content(app).unwrap() + &html_input_element::value(input)
}

/// Optional arguments give more forms: `encode_with_input(e, text)` is
/// `e.encode(text)`, and a union is its untagged enum (ADR 0215):
/// `decode_with_input(d, bytes.into())` is `d.decode(bytes)`.
pub fn round_trip(text: &str) -> (u32, String) {
    let bytes = text_encoder::encode_with_input(text_encoder::new(), text);
    let back = text_decoder::decode_with_input(text_decoder::new_with_label("utf-8"), bytes.into());
    (uint8_array::length(bytes), back)
}

/// A `BodyInit` of a `Blob`, and a `RequestInfo` of a URL: each the value
/// itself, as TypeScript's union takes it.
pub fn bodies(url: &str) -> (&'static webapi::Response, js::Promise<&'static webapi::Response>) {
    let blob = webapi::blob::new();
    let response = webapi::response::new_with_body(blob.into());
    (response, window::fetch(window, url.into()))
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
pub fn from_frame(frame: &webapi::HtmlIFrameElement, e: &Event) -> (bool, f64) {
    let sender = webapi::message_event::source(webapi::message_event::unchecked_from(e));
    let window_of = webapi::html_i_frame_element::content_window(frame);
    (js::object::is(&sender, &window_of), webapi::performance::now(window::performance(window)))
}

/// A canvas, which a library like canvas-confetti draws on: its size, set
/// and read, `canvas.width = 320`.
pub fn canvas_size() -> (u32, u32) {
    let canvas = webapi::html_canvas_element::unchecked_from(document::create_element(document, "canvas"));
    webapi::html_canvas_element::set_width(canvas, 320);
    webapi::html_canvas_element::set_height(canvas, 200);
    (webapi::html_canvas_element::width(canvas), webapi::html_canvas_element::height(canvas))
}
