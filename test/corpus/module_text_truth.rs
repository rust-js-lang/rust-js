// A module variable's text kept by its truth, `filter(|n| !n.is_empty())`,
// then matched, is tested by its truth, `name ? `..${name}..` : ".."`,
// where the arm that reads it does nothing; one whose arm runs code that
// changes the variable keeps its binding (ADR 0635, amended).

use std::cell::RefCell;

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static COMPONENT_NAME: RefCell<Option<String>> = const { RefCell::new(None) };
}

fn message() -> String {
    let message = match COMPONENT_NAME.with_borrow(Clone::clone).filter(|name| !name.is_empty()) {
        Some(component_name) => format!("in the <{component_name}> component"),
        None => "in one of your components".to_string(),
    };
    message
}

fn cleared_name(name: &str) -> String {
    COMPONENT_NAME.set(None);
    name.to_uppercase()
}

fn shouted() -> String {
    let shout = match COMPONENT_NAME.with_borrow(Clone::clone).filter(|name| !name.is_empty()) {
        Some(name) => cleared_name(&name),
        None => "-".to_string(),
    };
    shout
}

fn main() {
    println!("{}", message());
    COMPONENT_NAME.set(Some(String::new()));
    println!("{}", message());
    COMPONENT_NAME.set(Some("App".to_string()));
    println!("{} {} {}", message(), shouted(), shouted());
}
