// A thread-local's `with_borrow` and `with` give what their closure gives,
// an `Option` of an `Option` too, boxed as the closure makes it: React's
// SimpleEventPlugin reads a `string | null` its map may lack (rd111, ADR
// 0051).
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static NAMES: RefCell<HashMap<String, Option<String>>> = RefCell::new(HashMap::new());
    static LAST: Option<Option<u32>> = const { Some(None) };
}

fn register(dom: &str, react: Option<String>) {
    NAMES.with_borrow_mut(|names| {
        names.insert(dom.to_owned(), react);
    });
}

fn react_name_length(dom: &str) -> Option<usize> {
    let Some(react_name) = NAMES.with_borrow(|names| names.get(dom).cloned()) else {
        return None;
    };
    Some(react_name.map_or(0, |name| name.len()))
}

pub fn main() {
    register("click", Some("onClick".to_owned()));
    register("beforeblur", None);
    for dom in ["click", "beforeblur", "keydown"] {
        println!("{dom}: {:?}", react_name_length(dom));
    }
    println!("{:?}", LAST.with(|last| *last));
}
