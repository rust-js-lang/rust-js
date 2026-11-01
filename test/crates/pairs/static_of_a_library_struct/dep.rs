//! A library's struct of functions, React's `Dispatcher`, which a consumer
//! makes a static of, and a struct of a `Vec`, which a consumer's constant
//! of is a value of its own at each use.

pub struct Hooks {
    pub use_id: fn() -> String,
    pub use_memo: fn(fn() -> u32) -> u32,
    pub name: &'static str,
}

pub struct Bag {
    pub items: Vec<u32>,
}

pub fn render(hooks: &Hooks) -> String {
    format!("{} {} {}", hooks.name, (hooks.use_id)(), (hooks.use_memo)(|| 7))
}
