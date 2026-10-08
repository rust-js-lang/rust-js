//! Every JS form the webapi crate's bindings use (ADR 0024), for the test to
//! check in the generated JS.

use js::uint8_array;
use webapi::{Event, EventTargetExt, event, html_input_element, node, text_decoder, text_encoder, document, window};

pub fn forms() -> String {
    let input = html_input_element::unchecked_from(document.create_element_named("input"));
    input.set_value("typed");
    let app = document.get_element_by_id("app").expect("the page has an #app");
    app.append(input);
    app.append("!");
    let ping: &Event = event::new("ping");
    app.add_event_listener_named("ping", |e| e.prevent_default());
    let _ = window.dispatch_event(ping);
    app.text_content().unwrap() + &input.value()
}

/// Optional arguments give more forms: `encode_with_input(e, text)` is
/// `e.encode(text)`, and a union takes its member as it is (ADR 0229):
/// `decode_with_input(d, bytes)` is `d.decode(bytes)`.
pub fn round_trip(text: &str) -> (u32, String) {
    let bytes = text_encoder::new().encode_with_input(text);
    let back = text_decoder::new_with_label("utf-8").decode_with_input(bytes);
    (uint8_array::length(bytes), back)
}

/// A `BodyInit` of a `Blob`, and a `RequestInfo` of a URL: each the value
/// itself, as TypeScript's union takes it.
pub fn bodies(url: &str) -> (&'static webapi::Response, js::Promise<&'static webapi::Response>) {
    let blob = webapi::blob::new();
    let response = webapi::response::new_with_body(blob);
    (response, window.fetch(url))
}

/// A `sequence` is a slice (ADR 0219): a `Blob` of its parts, each a
/// `BlobPart`, and the clipboard written a list of items, each `new
/// Blob([text, "!"])` and `navigator.clipboard.write(items)`.
pub fn copied(text: &str, items: &[&webapi::ClipboardItem]) -> (&'static webapi::Blob, js::Promise<()>) {
    let blob = webapi::blob::new_with_blob_parts(&[text.into(), "!".into()]);
    (blob, webapi::navigator.clipboard().write(items))
}

/// A frame's window, a message's sender, and the page's clock:
/// `frame.contentWindow`, `e.source` and `window.performance.now()`.
pub fn from_frame(frame: &webapi::HTMLIFrameElement, e: &Event) -> (bool, f64) {
    let sender = webapi::message_event::unchecked_from(e).source();
    let window_of = frame.content_window();
    (js::object::is(&sender, &window_of), window.performance().now())
}

/// A canvas, which a library like canvas-confetti draws on: its size, set
/// and read, `canvas.width = 320`.
pub fn canvas_size() -> (u32, u32) {
    let canvas = webapi::html_canvas_element::unchecked_from(document.create_element_named("canvas"));
    canvas.set_width(320);
    canvas.set_height(200);
    (canvas.width(), canvas.height())
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
    let names = el.get_attribute_names();
    let languages = webapi::navigator.languages();
    let into = text_encoder::new().encode_into("hi", bytes);
    (names, languages.len(), blob.size(), param.value(), node::ELEMENT_NODE, into.written)
}

/// An event handler property, `onclick`: a closure given the event its name
/// is on the target, a button's click a `PointerEvent`, or `None`, `null`.
pub fn handlers(button: &webapi::HTMLButtonElement) -> bool {
    button.set_onclick(Some(Box::new(|e: &webapi::PointerEvent| {
            e.prevent_default();
        })));
    let set = button.onclick().is_some();
    button.set_onclick(None);
    set
}

/// A constructor of a class from a spec the crate didn't read before.
pub fn socket(url: &str) -> &'static webapi::WebSocket {
    webapi::web_socket::new(url)
}

/// A CSS property of an element's style, by its own name, as TypeScript has
/// each: `style.backgroundColor`.
pub fn styled(el: &webapi::HTMLElement) -> String {
    let style = el.style();
    style.set_background_color("red");
    style.webkit_line_clamp()
}

/// What an iterable, a maplike or a setlike declares: `forEach` of each
/// item, and a map's `get`, `has` and `size`.
pub fn iterables(nodes: &webapi::NodeList, headers: &webapi::Headers, ranges: &webapi::HighlightRegistry) -> (u32, Option<String>, bool, u32) {
    let mut count = 0;
    nodes.for_each(Box::new(move |_node: &webapi::Node, _i: u32, _list: &webapi::NodeList| {
        count += 1;
    }));
    let mut seen = 0;
    headers.for_each(Box::new(move |_value: &str, _name: &str, _headers: &webapi::Headers| {
        seen += 1;
    }));
    (seen, headers.get("a"), ranges.has("x"), ranges.size())
}

/// What an iterable gives, `keys()`, `values()` and `entries()`: JS
/// iterators, each a `Box<dyn Iterator>` (ADR 0140), stepped by `for`,
/// adapted lazily by JS's iterator helpers, or stepped by `next()`.
pub fn iterated(headers: &webapi::Headers, list: &webapi::DOMTokenList) -> (Vec<String>, usize, Option<String>) {
    let mut names = Vec::new();
    for (name, value) in headers.entries() {
        names.push(format!("{name}={value}"));
    }
    let long = list.values().filter(|token| token.len() > 3).count();
    let mut keys = headers.keys();
    (names, long, keys.next())
}
