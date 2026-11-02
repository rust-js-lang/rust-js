// A line comment after an array's statement, on its line, is after the
// statement, `const END_KEYCODES = [9, 13, 27, 32]; // Tab, Return, Esc,
// Space`, as React DOM's BeforeInputEventPlugin writes it, though the JS
// writes such an array an item a line before laying it out (rd605); one
// after an item, on its line, is after that item (rd344a).
const END_KEYCODES: [f64; 4] = [9.0, 13.0, 27.0, 32.0]; // Tab, Return, Esc, Space

static TRACK_NAMES: [&str; 3] = [
    "Primary",
    "Parallel", // Padded to give each track a unique name.
    "Parallel\u{200b}",
];

/// Of an object, a comment after a field whose Rust closes what the JS
/// doesn't have, `Cell::new(None)`'s `)`, is after the field, not the
/// object's `}`.
struct CacheNode {
    status: u32,
    object: std::cell::Cell<Option<u32>>,
}

fn cache_node() -> CacheNode {
    CacheNode {
        status: 0,                          // status
        object: std::cell::Cell::new(None), // object cache
    }
}

fn is_end(code: f64) -> bool {
    END_KEYCODES.contains(&code)
}

fn tabs() -> [u32; 3] {
    [9, 9, 9] // a block's last, before its `}`
}

fn main() {
    let codes = [9.0, 10.0, 32.0, 33.0]; // a few to test
    for code in codes {
        println!("{code}: {}", is_end(code));
    }
    let node = cache_node();
    println!("{:?} {:?} {} {:?}", TRACK_NAMES, tabs(), node.status, node.object.get());
}
