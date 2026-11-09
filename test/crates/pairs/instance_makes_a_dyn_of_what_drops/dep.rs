//! `node`, which its consumer compiles itself as it's given a closure that
//! drops, makes a `dyn Fn` of it there: one that drops it, as the library's
//! own `dyn Fn`s do (#1051).
pub struct Tag(pub &'static str);

impl Drop for Tag {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub struct Node {
    pub run: Box<dyn Fn() -> u32>,
}

pub fn node(run: impl Fn() -> u32 + 'static) -> Node {
    Node { run: Box::new(run) }
}

/// The library's own `dyn Fn` that drops what it holds.
pub fn tagged() -> Box<dyn Fn() -> u32> {
    let tag = Tag("tagged");
    Box::new(move || {
        let tag = &tag;
        tag.0.len() as u32
    })
}
