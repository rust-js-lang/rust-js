// A counter, written against the DOM through the `webapi` crate: bindings
// generated from W3C's WebIDL (ADR 0024). Also closures (ADR 0022), and
// strings, references and shared state (ADR 0023).

use std::cell::Cell;
use std::rc::Rc;

// Each DOM interface is a type whose members are its methods, and each tag
// and event's name a type whose value is its string (ADR 0223):
// `document.create_element(Button)` is `document.createElement("button")`,
// an `HTMLButtonElement`.
use webapi::{document};
use webapi::events::Click;
use webapi::tags::{Button, Div, Output};
use webapi::{Element, EventTargetExt, HTMLButtonElement, HTMLOutputElement};

fn button(label: &str) -> &'static HTMLButtonElement {
    let b = document.create_element(Button);
    b.set_text_content(label);
    b
}

/// A button that adds `by` to the shared count, and shows the new count.
fn stepper(label: &str, by: i32, count: &Rc<Cell<i32>>, output: &'static HTMLOutputElement) -> &'static HTMLButtonElement {
    let b = button(label);
    let count = count.clone();
    b.add_event_listener(Click, move |_| {
        count.set(count.get() + by);
        output.set_text_content(&count.get().to_string());
    });
    b
}

pub fn main() {
    let app = document.get_element_by_id("app").expect("the page has an #app");
    // Both buttons change one count, so they share it: `Rc` to share, `Cell`
    // to change it through a shared reference.
    let count = Rc::new(Cell::new(0));
    let output = document.create_element(Output);
    output.set_text_content("0");
    // An `Element` is a `Node` (`Deref`), so it goes where `append` wants a `Node`.
    app.append(stepper("-", -1, &count, output));
    app.append(output);
    app.append(stepper("+", 1, &count, output));
}

// Tests, in Rust (ADR 0026): `rust-js --test` compiles them, and `bun test`
// runs them in happy-dom's DOM.
#[cfg(test)]
mod tests {
    use super::*;
    use webapi::{HTMLElement, html_element};

    /// An empty page with the `<div id="app">` that `main` looks for.
    fn page() -> &'static Element {
        let body = document.body().unwrap();
        body.set_text_content("");
        let app = document.create_element(Div);
        app.set_id("app");
        body.append(app);
        app
    }

    fn nth_button(app: &Element, n: u32) -> &'static HTMLElement {
        html_element::unchecked_from(app.query_selector_all("button").item(n).unwrap())
    }

    fn shown(app: &Element) -> String {
        app.query_selector("output").unwrap().text_content().unwrap()
    }

    #[test]
    fn starts_at_zero() {
        let app = page();
        main();
        assert_eq!(app.text_content().unwrap(), "-0+");
    }

    #[test]
    fn both_buttons_change_one_count() {
        let app = page();
        main();
        let (minus, plus) = (nth_button(app, 0), nth_button(app, 1));
        plus.click();
        plus.click();
        plus.click();
        minus.click();
        assert_eq!(shown(app), "2");
        minus.click();
        minus.click();
        minus.click();
        assert_eq!(shown(app), "-1");
    }

    /// Needs real layout, which happy-dom doesn't do (every box is empty there),
    /// so it only runs in a browser: `-- --cfg browser` (ADR 0027).
    #[test]
    #[cfg_attr(not(browser), ignore = "needs a real browser")]
    fn the_count_sits_between_the_buttons() {
        let app = page();
        main();
        let minus = nth_button(app, 0).get_bounding_client_rect();
        let output = app.query_selector("output").unwrap().get_bounding_client_rect();
        let plus = nth_button(app, 1).get_bounding_client_rect();
        assert!(minus.width() > 0.0, "the buttons have a size");
        assert!(minus.right() <= output.left());
        assert!(output.right() <= plus.left());
    }
}
