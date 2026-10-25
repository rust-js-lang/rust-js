// A numbered enum's variant named by `#[rust_js::name]` is its module's
// `const` of that name, as React's `ReactCacheImpl` names its statuses,
// `const UNTERMINATED = 0;` (ADR 0722, amended): still its number, tested,
// matched, compared, cast and kept in a field as Rust's, its `{:?}` the
// variant's Rust name.

use std::cell::Cell;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum Status {
    #[cfg_attr(rust_js, rust_js::name = "UNTERMINATED")]
    Unterminated = 0,
    #[cfg_attr(rust_js, rust_js::name = "TERMINATED")]
    Terminated = 1,
    #[cfg_attr(rust_js, rust_js::name = "ERRORED")]
    Errored = 2,
}

struct CacheNode {
    s: Cell<Status>,
    v: Cell<u32>,
}

fn create_cache_node() -> CacheNode {
    CacheNode {
        s: Cell::new(Status::Unterminated),
        v: Cell::new(0),
    }
}

fn read(node: &CacheNode) -> Result<u32, String> {
    if node.s.get() == Status::Terminated {
        return Ok(node.v.get());
    }
    if node.s.get() == Status::Errored {
        return Err(format!("errored with {}", node.v.get()));
    }
    Err("pending".to_string())
}

fn describe(status: Status) -> &'static str {
    match status {
        Status::Unterminated => "unterminated",
        Status::Terminated => "terminated",
        Status::Errored => "errored",
    }
}

fn main() {
    let node = create_cache_node();
    println!("{:?} {:?}", node.s.get(), read(&node));
    node.v.set(7);
    node.s.set(Status::Terminated);
    println!("{:?} {:?}", node.s.get(), read(&node));
    let failed = create_cache_node();
    failed.v.set(3);
    failed.s.set(Status::Errored);
    println!("{:?}", read(&failed));
    for status in [Status::Errored, Status::Unterminated, Status::Terminated] {
        println!("{status:?} {} {}", describe(status), status as u8);
    }
    println!(
        "{} {} {}",
        Status::Terminated < Status::Errored,
        node.s.get() == failed.s.get(),
        Status::Errored as u8 + 1
    );
}
