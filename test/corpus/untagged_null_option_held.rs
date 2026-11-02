// An `Option` of an untagged enum one of whose variants is `null`,
// `#[rust_js::null]`, and none `undefined`, held where it's made and read
// there, is the enum's value itself, its `None` `undefined` (ADR 1017,
// amended): a map's `get(key)`, and its `cloned()`, are JS's
// `cache.get(key)`, tested `=== undefined`, its `null` its variant's, as
// React's Flight client keeps its chunk cache, a `Map<string, null |
// Promise>`. One given to a function, compared, or of an enum whose clone
// copies, keeps its box; and so does one of an enum that's `undefined` too,
// and an `Option` of an `Option`'s map, whose `None` it holds as
// `undefined`.

use std::cell::RefCell;
use std::collections::HashMap;

#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Clone, Copy, Debug, PartialEq)]
enum Entry {
    Pending(&'static str),
    Count(f64),
    #[cfg_attr(rust_js, rust_js::null)]
    Loaded,
}

// One whose payload a clone copies, a `Vec`'s.
#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Clone, Debug, PartialEq)]
enum Slot {
    Items(Vec<i32>),
    #[cfg_attr(rust_js, rust_js::null)]
    Cleared,
}

// One whose payload a clone copies and nothing drops, an object's.
#[derive(Clone, Debug, PartialEq)]
struct Point {
    x: f64,
}

#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Clone, Debug, PartialEq)]
enum Shape {
    At(Point),
    #[cfg_attr(rust_js, rust_js::null)]
    Nowhere,
}

// One that's `undefined` too, whose `Some(Missing)` no `undefined` could
// tell from `None`.
#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Clone, Copy, Debug, PartialEq)]
enum Field {
    Count(f64),
    #[cfg_attr(rust_js, rust_js::undefined)]
    Missing,
    #[cfg_attr(rust_js, rust_js::null)]
    Cleared,
}

// One that's neither, whose `Option` is the value or `undefined`
// anywhere, tested `!= null` as any `Option` is.
#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Clone, Copy, Debug, PartialEq)]
enum Plain {
    Text(&'static str),
    Count(f64),
}

thread_local! {
    static CACHE: RefCell<HashMap<String, Entry>> = RefCell::new(HashMap::new());
}

fn describe(entry: &Entry) -> String {
    match entry {
        Entry::Pending(name) => format!("pending {name}"),
        Entry::Count(n) => format!("count {n}"),
        Entry::Loaded => "done".to_string(),
    }
}

// Upstream's `const entry = chunkCache.get(chunkId); if (entry ===
// undefined) {..} else if (entry !== null) { promises.push(entry); }`.
fn preload(cache: &HashMap<String, Entry>, id: &str, pending: &mut Vec<&'static str>) -> bool {
    let entry = cache.get(id).cloned();
    match entry {
        None => false,
        Some(Entry::Pending(name)) => {
            pending.push(name);
            true
        }
        Some(Entry::Count(_)) | Some(Entry::Loaded) => true,
    }
}

fn shown(cache: &HashMap<String, Entry>, id: &str) -> String {
    match cache.get(id) {
        None => "missing".to_string(),
        Some(Entry::Loaded) => "loaded".to_string(),
        Some(entry) => describe(entry),
    }
}

fn filled(cache: &HashMap<String, Entry>, id: &str, fallback: Entry) -> Entry {
    let mut entry = cache.get(id).copied();
    if entry.is_none() {
        entry = Some(fallback);
    }
    match entry {
        Some(entry) => entry,
        None => Entry::Loaded,
    }
}

fn given(entry: Option<&Entry>) -> String {
    format!("{entry:?}")
}

fn owned(entry: Option<Entry>) -> String {
    format!("{entry:?}")
}

// A clone given on, boxed, of one held.
fn kept(cache: &HashMap<String, Entry>, id: &str) -> String {
    let entry = cache.get(id).cloned();
    owned(entry)
}

fn same(cache: &HashMap<String, Entry>, a: &str, b: &str) -> bool {
    cache.get(a) == cache.get(b)
}

fn field(fields: &HashMap<u32, Field>, at: u32) -> &'static str {
    match fields.get(&at) {
        None => "none",
        Some(Field::Missing) => "missing",
        Some(Field::Cleared) => "cleared",
        Some(Field::Count(_)) => "count",
    }
}

fn plain(plains: &HashMap<u32, Plain>, at: u32) -> &'static str {
    match plains.get(&at) {
        None => "none",
        Some(Plain::Text(text)) => text,
        Some(Plain::Count(_)) => "count",
    }
}

fn slot(slots: &HashMap<u32, Slot>, at: u32) -> usize {
    match slots.get(&at).cloned() {
        None => 0,
        Some(Slot::Items(items)) => items.len(),
        Some(Slot::Cleared) => 100,
    }
}

// A clone changed, of one whose clone copies: the map's is as it was.
fn grown(slots: &HashMap<u32, Slot>, at: u32) -> usize {
    match slots.get(&at).cloned() {
        Some(Slot::Items(mut items)) => {
            items.push(0);
            items.len()
        }
        _ => 0,
    }
}

fn moved(shapes: &HashMap<u32, Shape>, at: u32) -> f64 {
    match shapes.get(&at).cloned() {
        Some(Shape::At(mut point)) => {
            point.x += 1.0;
            point.x
        }
        Some(Shape::Nowhere) => -1.0,
        None => 0.0,
    }
}

fn nested(values: &HashMap<u32, Option<u32>>, at: u32) -> u32 {
    match values.get(&at) {
        None => 0,
        Some(None) => 1,
        Some(Some(n)) => *n,
    }
}

fn main() {
    let mut cache = HashMap::new();
    cache.insert("a".to_string(), Entry::Pending("a.js"));
    cache.insert("b".to_string(), Entry::Loaded);
    cache.insert("c".to_string(), Entry::Count(3.0));
    let mut pending = Vec::new();
    for id in ["a", "b", "c", "d"] {
        let preloaded = preload(&cache, id, &mut pending);
        let filled = filled(&cache, id, Entry::Count(0.0));
        println!("{id} {preloaded} {} {filled:?}", shown(&cache, id));
        println!("{} {} {}", given(cache.get(id)), kept(&cache, id), same(&cache, id, "b"));
    }
    println!("{pending:?}");
    CACHE.with_borrow_mut(|cache| cache.insert("x".to_string(), Entry::Loaded));
    let cached = CACHE.with_borrow(|cache| preload(cache, "x", &mut pending));
    println!("{cached}");
    let mut slots = HashMap::new();
    slots.insert(1, Slot::Items(vec![1, 2]));
    slots.insert(2, Slot::Cleared);
    println!("{} {} {}", slot(&slots, 1), slot(&slots, 2), slot(&slots, 3));
    println!("{} {}", grown(&slots, 1), slot(&slots, 1));
    let shapes = HashMap::from([(1, Shape::At(Point { x: 1.0 })), (2, Shape::Nowhere)]);
    println!("{} {} {} {:?}", moved(&shapes, 1), moved(&shapes, 2), moved(&shapes, 3), shapes.get(&1));
    let fields = HashMap::from([(1, Field::Count(1.0)), (2, Field::Missing), (3, Field::Cleared)]);
    println!("{} {} {} {}", field(&fields, 1), field(&fields, 2), field(&fields, 3), field(&fields, 4));
    let plains = HashMap::from([(1, Plain::Text("text")), (2, Plain::Count(2.0))]);
    println!("{} {} {}", plain(&plains, 1), plain(&plains, 2), plain(&plains, 3));
    let mut values = HashMap::new();
    values.insert(1, None);
    values.insert(2, Some(7));
    println!("{} {} {}", nested(&values, 1), nested(&values, 2), nested(&values, 3));
}
