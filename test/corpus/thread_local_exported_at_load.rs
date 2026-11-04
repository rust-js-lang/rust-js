// A `pub` thread-local is made as its module loads, as JS's `export const`
// is (ADR 1006, the user's decision), and a private one too (ADR 1006,
// amended), where making it then can't show (#554): React DOM's `export
// const ANIMATION_END = getVendorPrefixedEventName('animationend')`, which
// upstream's JS imports, whose initializer reads and fills a cache only the
// loading initializers touch. One whose initializer shows it would be made
// where it's first used, a `pub` one refused (`thread_local_lazy_where_observable`).
mod prefixes {
    use std::cell::RefCell;
    use std::collections::HashMap;

    thread_local! {
        static PREFIXED: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
        // Made as its module loads too, before the `pub` ones, whose
        // module imports its own: so `prefixed` reads it first.
        static VENDOR: String = vendor();
    }

    fn vendor() -> String {
        "webkit".to_owned()
    }

    pub fn prefixed(name: &str) -> String {
        if let Some(prefixed) = PREFIXED.with_borrow(|prefixed| prefixed.get(name).cloned()) {
            return prefixed;
        }
        VENDOR.with(|vendor| {
            let prefixed = vendor.clone() + name;
            PREFIXED.with_borrow_mut(|cache| cache.insert(name.to_owned(), prefixed.clone()));
            prefixed
        })
    }
}

mod event_names {
    use super::prefixes::prefixed;

    thread_local! {
        pub static ANIMATION_END: String = prefixed("animationend");
        pub static TRANSITION_END: String = prefixed("transitionend");
        // Of its own module's function, which reads a thread-local written
        // after it: made after that one.
        pub static FIRST: usize = first_length();
    }

    fn first_length() -> usize {
        LENGTHS.with(|lengths| lengths[0])
    }

    thread_local! {
        static LENGTHS: Vec<usize> = vec![4, 2];
    }
}

fn main() {
    println!("{}", event_names::ANIMATION_END.with(Clone::clone));
    println!("{}", event_names::TRANSITION_END.with(|end| end.len()));
    println!("{}", event_names::FIRST.with(|first| *first));
}
