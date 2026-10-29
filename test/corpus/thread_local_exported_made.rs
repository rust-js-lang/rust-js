// A `pub` thread-local is its value, made as its module loads, where its
// initializer only makes one; a `pub(crate)` one whose initializer runs the
// crate's code is made as its module loads too, which the crate's modules
// read by its name, and a `pub fn` gives JS its value (ADR 1006, amended):
// `main` first reads it, as its module made it.
mod event_names {
    use std::cell::RefCell;

    thread_local! {
        static PREFIXED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    pub fn prefixed(name: &str) -> String {
        println!("prefixing {name}");
        PREFIXED.with_borrow_mut(|prefixed| prefixed.push(name.to_owned()));
        "webkit".to_owned() + name
    }

    pub fn count() -> usize {
        PREFIXED.with_borrow(Vec::len)
    }

    thread_local! {
        pub static ANIMATION: &'static str = "animation";
        pub(crate) static ANIMATION_END: String = prefixed("animationend");
    }

    pub fn animation_end() -> String {
        ANIMATION_END.with(Clone::clone)
    }
}

fn main() {
    println!("{}", event_names::ANIMATION_END.with(|end| end.len()));
    println!("main {}", event_names::count());
    println!("{}", event_names::ANIMATION.with(|a| *a));
    println!("{} {}", event_names::animation_end(), event_names::count());
}
