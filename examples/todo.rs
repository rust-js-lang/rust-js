// A todo list, written against the DOM through the `webapi` crate (ADR 0024).
// The todos live in one `Rc<RefCell<State>>`; after every change, the list
// is drawn again from it (ADR 0025).

use std::cell::RefCell;
use std::rc::Rc;

use webapi::{document};
use webapi::events::Keydown;
use webapi::{Element, EventTargetExt, html_element, html_input_element};

#[derive(Clone, Copy, PartialEq)]
enum Filter {
    All,
    Active,
    Completed,
}

struct Todo {
    id: u32,
    title: String,
    done: bool,
}

struct State {
    todos: Vec<Todo>,
    next_id: u32,
    filter: Filter,
}

type Shared = Rc<RefCell<State>>;

/// The parts of the page that change.
#[derive(Clone, Copy)]
struct View {
    list: &'static Element,
    left: &'static Element,
}

/// An element by its tag's name: `create_element(document, Button)` is one by
/// its tag's type, `HTMLButtonElement` (ADR 0223).
fn create(tag: &str) -> &'static Element {
    document.create_element_named(tag)
}

fn text(tag: &str, s: &str) -> &'static Element {
    let e = create(tag);
    e.set_text_content(s);
    e
}

/// When `event` fires on `target`, apply `change` to the state and draw it again.
fn on(target: &'static Element, event: &str, state: &Shared, view: View, change: Box<dyn Fn(&mut State)>) {
    let state = state.clone();
    target.add_event_listener_named(event, move |_| {
        change(&mut state.borrow_mut());
        render(&state, view);
    });
}

fn add(s: &mut State, title: &str) {
    s.todos.push(Todo { id: s.next_id, title: title.to_string(), done: false });
    s.next_id += 1;
}

fn toggle(s: &mut State, id: u32) {
    for todo in &mut s.todos {
        if todo.id == id {
            todo.done = !todo.done;
        }
    }
}

fn shown(filter: Filter, todo: &Todo) -> bool {
    match filter {
        Filter::All => true,
        Filter::Active => !todo.done,
        Filter::Completed => todo.done,
    }
}

/// One `<li>`: a checkbox, the title, and a button to delete it.
fn item(state: &Shared, view: View, todo: &Todo) -> &'static Element {
    let li = create("li");
    let id = todo.id;

    let check = html_input_element::unchecked_from(create("input"));
    check.set_type("checkbox");
    check.set_checked(todo.done);
    on(check, "change", state, view, Box::new(move |s| toggle(s, id)));

    let title = html_element::unchecked_from(text("span", &todo.title));
    if todo.done {
        title.style().set_property("text-decoration", "line-through");
    }

    let delete = text("button", "×");
    on(delete, "click", state, view, Box::new(move |s| s.todos.retain(|t| t.id != id)));

    li.append(check);
    li.append(title);
    li.append(delete);
    li
}

fn render(state: &Shared, view: View) {
    let s = state.borrow();
    view.list.set_text_content("");
    let mut left = 0;
    for todo in &s.todos {
        if !todo.done {
            left += 1;
        }
        if shown(s.filter, todo) {
            view.list.append(item(state, view, todo));
        }
    }
    let noun = if left == 1 { " item left" } else { " items left" };
    view.left.set_text_content(&(left.to_string() + noun));
}

pub fn main() {
    let app = document.get_element_by_id("app").expect("the page has an #app");
    let state: Shared = Rc::new(RefCell::new(State { todos: Vec::new(), next_id: 1, filter: Filter::All }));
    let view = View { list: create("ul"), left: create("span") };

    // Typing a title and pressing Enter adds it.
    let input = html_input_element::unchecked_from(create("input"));
    input.set_placeholder("What needs to be done?");
    let adding = state.clone();
    input.add_event_listener(Keydown, move |e| {
        let title = input.value();
        let title = title.trim();
        if e.key() == "Enter" && !title.is_empty() {
            add(&mut adding.borrow_mut(), title);
            input.set_value("");
            render(&adding, view);
        }
    });

    // Which todos to show, and clearing the completed ones.
    let footer = create("p");
    footer.append(view.left);
    for (label, filter) in [("All", Filter::All), ("Active", Filter::Active), ("Completed", Filter::Completed)] {
        let b = text("button", label);
        on(b, "click", &state, view, Box::new(move |s| s.filter = filter));
        footer.append(b);
    }
    let clear = text("button", "Clear completed");
    on(clear, "click", &state, view, Box::new(|s| s.todos.retain(|t| !t.done)));
    footer.append(clear);

    app.append(input);
    app.append(view.list);
    app.append(footer);
    render(&state, view);
}

// Tests, in Rust (ADR 0026): `rust-js --test` compiles them, and `bun test`
// runs them in happy-dom's DOM.
#[cfg(test)]
mod tests {
    use super::*;
    use webapi::{Event, HTMLInputElement};

    /// `new KeyboardEvent(type, { key })`. The webapi crate can't take
    /// dictionaries yet, but a struct is a JS object with the same fields
    /// (ADR 0020), so it can be declared here (ADR 0021).
    #[allow(dead_code)] // `key` is read by JS, not by Rust.
    struct KeyboardEventInit {
        key: String,
    }

    unsafe extern "Rust" {
        #[link_name = "new KeyboardEvent"]
        safe fn keyboard_event(type_: &str, init: &KeyboardEventInit) -> &'static Event;
    }

    /// An empty page with the `<div id="app">` that `main` looks for.
    fn page() -> &'static Element {
        let body = document.body().unwrap();
        body.set_text_content("");
        let app = create("div");
        app.set_id("app");
        body.append(app);
        main();
        app
    }

    fn input(app: &Element) -> &'static HTMLInputElement {
        html_input_element::unchecked_from(app.query_selector("input").unwrap())
    }

    /// Type `title` and press `key`.
    fn type_in(app: &Element, title: &str, key: &str) {
        let field = input(app);
        field.set_value(title);
        field.dispatch_event(keyboard_event("keydown", &KeyboardEventInit { key: key.to_string() }));
    }

    fn titles(app: &Element) -> Vec<String> {
        let spans = app.query_selector_all("li span");
        let mut titles = Vec::new();
        for i in 0..spans.length() {
            titles.push(spans.item(i).unwrap().text_content().unwrap());
        }
        titles
    }

    fn left(app: &Element) -> String {
        app.query_selector("p span").unwrap().text_content().unwrap()
    }

    /// The `n`th element matching `selector`, to click.
    fn nth(app: &Element, selector: &str, n: u32) -> &'static webapi::HTMLElement {
        html_element::unchecked_from(app.query_selector_all(selector).item(n).unwrap())
    }

    #[test]
    fn starts_empty() {
        let app = page();
        assert_eq!(titles(app), Vec::<String>::new());
        assert_eq!(left(app), "0 items left");
    }

    #[test]
    fn enter_adds_a_trimmed_title() {
        let app = page();
        type_in(app, "  Buy milk  ", "Enter");
        assert_eq!(titles(app), ["Buy milk"]);
        assert_eq!(input(app).value(), "");
        assert_eq!(left(app), "1 item left");
    }

    #[test]
    fn blank_titles_and_other_keys_add_nothing() {
        let app = page();
        type_in(app, "   ", "Enter");
        type_in(app, "Not yet", "a");
        assert_eq!(titles(app), Vec::<String>::new());
    }

    #[test]
    fn ticking_one_off_crosses_it_out() {
        let app = page();
        type_in(app, "Buy milk", "Enter");
        type_in(app, "Walk the dog", "Enter");
        nth(app, "li input", 0).click();
        assert_eq!(left(app), "1 item left");
        let title = nth(app, "li span", 0);
        assert_eq!(title.style().get_property_value("text-decoration"), "line-through");
    }

    #[test]
    fn filters_clear_and_delete() {
        let app = page();
        type_in(app, "Buy milk", "Enter");
        type_in(app, "Walk the dog", "Enter");
        nth(app, "li input", 0).click();
        let (all, active, completed, clear) = (nth(app, "p button", 0), nth(app, "p button", 1), nth(app, "p button", 2), nth(app, "p button", 3));
        active.click();
        assert_eq!(titles(app), ["Walk the dog"]);
        completed.click();
        assert_eq!(titles(app), ["Buy milk"]);
        all.click();
        assert_eq!(titles(app), ["Buy milk", "Walk the dog"]);
        clear.click();
        assert_eq!(titles(app), ["Walk the dog"]);
        nth(app, "li button", 0).click();
        assert_eq!(titles(app), Vec::<String>::new());
        assert_eq!(left(app), "0 items left");
    }
}
