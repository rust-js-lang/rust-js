//! A call through the library's fn pointers, which may only be its own Rust
//! functions, as React DOM's `Children.forEach(props.children, fn)`.
thread_local! {
    // Made where it's first used, as what the library's function runs is
    // its own, which may be anything: what it's given prints after `main`.
    static FIRST: u32 = dep::CHILDREN.with(|children| {
        (children.for_each)(Some("x"), &|i| println!("made {i}"), None);
        1
    });
}

pub fn main() {
    println!("main");
    println!("{}", FIRST.with(|first| *first));
    dep::CHILDREN.with(|children| {
        (children.for_each)(Some("ab"), &|i| println!("child {i}"), None);
        (children.for_each)(Some("c"), &|i| println!("only {i}"), Some("option"));
        println!("{}", (children.count)(None));
    });
    let hooks = dep::hooks(add);
    println!("{}", (hooks.on_each)(1, None));
}

fn add(a: u32, b: Option<u32>) -> u32 {
    a + b.unwrap_or(2)
}
