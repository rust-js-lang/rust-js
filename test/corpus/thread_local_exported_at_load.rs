// A `pub` thread-local is made as its module loads, as JS's `export const`
// is, whatever its initializer runs (ADR 1006, the user's decision), and a
// private one too (ADR 1006, amended): React DOM's `export const ANIMATION_END =
// getVendorPrefixedEventName('animationend')`, which upstream's JS imports.
// It differs from Rust only in when its initializer runs, so `main` first
// reads each in the order their modules make them, and what each
// initializer did, and in which order, is the same.
mod prefixes {
    use std::cell::RefCell;

    thread_local! {
        static SEEN: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
        // Made as its module loads too, before the `pub` ones, whose
        // module imports its own: so `prefixed` reads it first.
        static VENDOR: String = vendor();
    }

    fn vendor() -> String {
        SEEN.with_borrow_mut(|seen| seen.push("vendor".to_owned()));
        "webkit".to_owned()
    }

    pub fn prefixed(name: &str) -> String {
        let vendor = VENDOR.with(Clone::clone);
        SEEN.with_borrow_mut(|seen| seen.push(name.to_owned()));
        vendor + name
    }

    pub fn seen() -> Vec<String> {
        SEEN.with_borrow(Clone::clone)
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
    println!("{:?}", prefixes::seen());
}
