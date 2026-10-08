// Loading something with `fetch` (ADR 0029): each step is a promise from the
// web crate, `.await`ed in turn. The URL is a `data:` URL, so the example
// needs no server; any URL the page may fetch works the same way.

use js::spawn;
use webapi::events::Click;
use webapi::tags::{Button, Output};
use webapi::{EventTargetExt, Element, document, element, node, response, window};

const URL: &str = "data:text/plain,Hello from a fetch!";

async fn load(url: &str, output: &'static Element) {
    node::set_text_content(output, "Loading…");
    let response = window::fetch(window, url).await;
    let text = response::text(response).await;
    let status = response::status(response).to_string();
    node::set_text_content(output, &(status + " " + &text));
}

pub fn main() {
    let app = document::get_element_by_id(document, "app").expect("the page has an #app");
    let button = document::create_element(document, Button);
    node::set_text_content(button, "Fetch");
    let output = document::create_element(document, Output);
    button.add_event_listener(Click, move |_| {
        spawn(Box::new(load(URL, output)));
    });
    element::append(app, button);
    element::append(app, output);
}
