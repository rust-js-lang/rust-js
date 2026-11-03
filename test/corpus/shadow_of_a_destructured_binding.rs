// A `let` that shadows a binding with its clone, `let id = id.clone();`,
// is the binding itself where the clone is the same JS value, a string's,
// and nothing sets the binding again (ADR 0277): of a binding that an
// or-pattern's alike arms destructure, `const [id, chunks] = metadata;`
// (rd712), and of a loop's tuple's parts, as of a `let`'s. A clone
// of a struct is a copy still, which the closure changes apart from the
// binding.

use std::collections::HashMap;

#[derive(Clone, Debug)]
struct Point {
    x: i32,
    y: i32,
}

// Each variant's value is its array, an untagged enum's.
#[cfg_attr(rust_js, rust_js::untagged)]
enum Metadata {
    Async((String, &'static [String], String, bool)),
    Sync((String, &'static [String], String)),
}

#[cfg_attr(rust_js, rust_js::untagged)]
enum Placed {
    Here((Point, String, bool)),
    There((Point, String)),
}

fn run(f: impl FnOnce() -> usize) -> usize {
    f()
}

fn require(id: &str) -> usize {
    id.len()
}

// React's Flight client's preloadModule, as its port writes it.
fn preload(metadata: &Metadata, eager: bool) -> Option<usize> {
    let (Metadata::Async((id, chunks, ..)) | Metadata::Sync((id, chunks, ..))) = metadata;
    let mut pending = Vec::new();
    for chunk in *chunks {
        pending.push(chunk.len());
    }
    if eager {
        Some(require(id))
    } else if pending.is_empty() {
        None
    } else {
        let id = id.clone();
        Some(run(move || require(&id) + pending.len()))
    }
}

// The clone of a struct is its own object, which the closure moves.
fn moved(placed: &Placed) -> (i32, i32, usize) {
    let (Placed::Here((point, label, ..)) | Placed::There((point, label, ..))) = placed;
    let mut point = point.clone();
    let mut shift = move || {
        point.x += 10;
        point.y += point.x;
        point.x + point.y
    };
    let sum = shift();
    (sum, point_x(placed), label.len())
}

fn point_x(placed: &Placed) -> i32 {
    let (Placed::Here((point, ..)) | Placed::There((point, ..))) = placed;
    point.x
}

// A loop's tuple's parts, each cloned into a closure.
fn labels(names: &[(String, u32)]) -> Vec<String> {
    let mut make: Vec<Box<dyn Fn() -> String>> = Vec::new();
    for (name, count) in names {
        let name = name.clone();
        let count = *count;
        make.push(Box::new(move || format!("{name}:{count}")));
    }
    make.iter().map(|f| f()).collect()
}

// A map's entries, each a `for` of its key and value, cloned likewise.
fn entries(counts: &HashMap<String, u32>) -> Vec<String> {
    let mut make: Vec<Box<dyn Fn() -> String>> = Vec::new();
    for (name, count) in counts {
        let name = name.clone();
        let count = *count;
        make.push(Box::new(move || format!("{name}={count}")));
    }
    let mut made: Vec<String> = make.iter().map(|f| f()).collect();
    made.sort();
    made
}

fn main() {
    let chunks: &'static [String] = Vec::leak(vec!["c".to_string(), "dd".to_string()]);
    let a = Metadata::Async(("ab".to_string(), chunks, "n".to_string(), true));
    let s = Metadata::Sync(("abc".to_string(), &[], "n".to_string()));
    println!("{:?} {:?}", preload(&a, false), preload(&a, true));
    println!("{:?} {:?}", preload(&s, false), preload(&s, true));
    let here = Placed::Here((Point { x: 1, y: 2 }, "h".to_string(), true));
    let there = Placed::There((Point { x: 3, y: 4 }, "t".to_string()));
    println!("{:?} {:?}", moved(&here), moved(&there));
    let names = vec![("a".to_string(), 1), ("b".to_string(), 2)];
    println!("{:?}", labels(&names));
    let counts: HashMap<String, u32> = names.into_iter().collect();
    println!("{:?}", entries(&counts));
}
