// A countdown, with async code (ADR 0029): an `async fn` that `.await`s a
// timer between steps, started from a click with `spawn`.

use std::cell::Cell;
use std::rc::Rc;

use js::{Promise, set_timeout, spawn};
use webapi::document;
use webapi::events::Click;
use webapi::tags::{Button, Output};
use webapi::{EventTargetExt, Element};

/// A promise that resolves after `ms` milliseconds.
fn sleep(ms: u32) -> Promise<()> {
    js::promise::new(move |resolve, _| {
        set_timeout(Box::new(move || resolve(())), Some(f64::from(ms)));
    })
}

async fn count_down(output: &'static Element, from: u32) {
    let mut n = from;
    while n > 0 {
        output.set_text_content(&n.to_string());
        sleep(500).await;
        n -= 1;
    }
    output.set_text_content("Go!");
}

pub fn main() {
    let app = document.get_element_by_id("app").expect("the page has an #app");
    let start = document.create_element(Button);
    start.set_text_content("Start");
    let output = document.create_element(Output);
    // One countdown at a time.
    let running = Rc::new(Cell::new(false));
    start.add_event_listener(Click, move |_| {
        if running.get() {
            return;
        }
        running.set(true);
        let running = running.clone();
        spawn(Box::new(async move {
            count_down(output, 3).await;
            running.set(false);
        }));
    }, None);
    app.append((start, output));
}

#[cfg(test)]
mod tests {
    use super::*;
    use webapi::html_element;
    use webapi::tags::Div;

    /// An empty page with the `<div id="app">` that `main` looks for.
    fn page() -> &'static Element {
        let body = document.body().unwrap();
        body.set_text_content("");
        let app = document.create_element(Div);
        app.set_id("app");
        body.append((app,));
        app
    }

    /// A promise runs up to its first `.await` as soon as it's made, so the
    /// click shows the first number before the handler returns.
    #[test]
    fn a_click_shows_the_first_number_at_once() {
        let app = page();
        main();
        html_element::unchecked_from(app.query_selector("button").unwrap()).click();
        assert_eq!(app.query_selector("output").unwrap().text_content().unwrap(), "3");
    }
}
