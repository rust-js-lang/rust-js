// Loading something with `fetch` (ADR 0029): each step is a promise from the
// web crate, `.await`ed in turn. The URL is a `data:` URL, so the example
// needs no server; any URL the page may fetch works the same way.

use js::spawn;
use webapi::{document, window};
use webapi::events::Click;
use webapi::tags::{Button, Output};
use webapi::{EventTargetExt, Element};

const URL: &str = "data:text/plain,Hello from a fetch!";

async fn load(url: &str, output: &'static Element) {
    output.set_text_content("Loading…");
    let response = window.fetch(url).await;
    let text = response.text().await;
    let status = response.status().to_string();
    output.set_text_content(&(status + " " + &text));
}

pub fn main() {
    let app = document.get_element_by_id("app").expect("the page has an #app");
    let button = document.create_element(Button);
    button.set_text_content("Fetch");
    let output = document.create_element(Output);
    button.add_event_listener(Click, move |_| {
        spawn(Box::new(load(URL, output)));
    });
    app.append(button);
    app.append(output);
}
