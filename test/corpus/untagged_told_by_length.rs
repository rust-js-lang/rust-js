// An untagged enum of arrays whose types fix their lengths, told apart by
// `length`: React's Fizz server's replay node, `[name, key, children,
// slots]` or a boundary's `[name, key, children, slots, fallback,
// rootSegmentID]`, which upstream tells by `node.length === 4` (rd490).
// Each of whose values is an array it holds itself, so it's changed in
// place, as upstream grows a node its parent's children and its working
// map both hold to a boundary's and shrinks it back.
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
struct Element(Option<String>, f64, Vec<f64>, Option<f64>);

#[derive(Clone, Debug, PartialEq)]
struct Boundary(Option<String>, f64, Vec<f64>, Option<f64>, Option<f64>, f64);

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(rust_js, rust_js::untagged)]
enum ReplayNode {
    Element(Element),
    Boundary(Boundary),
}

fn is_element(node: &ReplayNode) -> bool {
    matches!(node, ReplayNode::Element(_))
}

fn root_segment(node: &ReplayNode) -> Option<f64> {
    match node {
        ReplayNode::Element(_) => None,
        ReplayNode::Boundary(boundary) => Some(boundary.5),
    }
}

fn key(node: &ReplayNode) -> f64 {
    match node {
        ReplayNode::Element(Element(_, key, ..)) | ReplayNode::Boundary(Boundary(_, key, ..)) => *key,
    }
}

// Tuples, through an `Rc`, the array it points at.
#[cfg_attr(rust_js, rust_js::untagged)]
enum Pair {
    Two(Rc<(String, f64)>),
    Three(Rc<(String, f64, f64)>),
}

fn sum(pair: &Pair) -> f64 {
    match pair {
        Pair::Two(two) => two.1,
        Pair::Three(three) => three.1 + three.2,
    }
}

// Beside what isn't an array: each told as an array of its length.
#[cfg_attr(rust_js, rust_js::untagged)]
enum Shape {
    Word(String),
    Point((f64, f64)),
    Box((f64, f64, f64, f64)),
    Items([f64; 3]),
}

fn area(shape: &Shape) -> f64 {
    match shape {
        Shape::Word(word) => word.len() as f64,
        Shape::Point(_) => 0.0,
        Shape::Box((x0, y0, x1, y1)) => (x1 - x0) * (y1 - y0),
        Shape::Items(items) => items.iter().sum(),
    }
}

// A 2-tuple beside a struct's object, which no array is.
struct Named {
    name: String,
}

#[cfg_attr(rust_js, rust_js::untagged)]
enum Entry {
    Pair((String, f64)),
    Named(Rc<Named>),
}

fn name(entry: &Entry) -> &str {
    match entry {
        Entry::Pair((name, _)) => name,
        Entry::Named(named) => &named.name,
    }
}

// A `#[repr(transparent)]` struct is its field, a number here, no array.
#[repr(transparent)]
struct Id(f64);

#[cfg_attr(rust_js, rust_js::untagged)]
enum Key {
    Name(String),
    Id(Id),
}

fn key_text(key: &Key) -> String {
    match key {
        Key::Name(name) => name.clone(),
        Key::Id(Id(id)) => format!("#{id}"),
    }
}

// Shared, as a parent's children and a working map hold one node.
type Shared = Rc<RefCell<ReplayNode>>;

// Upgraded to a boundary, the node its holders hold grown to six items;
// a boundary given another fallback and ID.
fn track(node: &Shared, fallback: Option<f64>, id: f64) {
    let mut node = node.borrow_mut();
    if let ReplayNode::Element(Element(name, key, children, slots)) = &*node {
        *node = ReplayNode::Boundary(Boundary(name.clone(), *key, children.clone(), *slots, fallback, id));
    } else if let ReplayNode::Boundary(boundary) = &mut *node {
        boundary.4 = fallback;
        boundary.5 = id;
    }
}

// Each written through a `&mut`, a field's write of each variant.
fn renumber(node: &mut ReplayNode, key: f64) {
    match node {
        ReplayNode::Element(element) => element.1 = key,
        ReplayNode::Boundary(boundary) => boundary.1 = key,
    }
}

// Downgraded to a plain node, shrunk back to four.
fn untrack(node: &Shared) {
    let mut node = node.borrow_mut();
    if let ReplayNode::Boundary(Boundary(name, key, ..)) = &*node {
        *node = ReplayNode::Element(Element(name.clone(), *key, vec![], None));
    }
}

// Shrunk to its first four, its fallback its slots: what's read of the
// items it loses is read before it loses them.
fn demote(node: &Shared) {
    let mut node = node.borrow_mut();
    if let ReplayNode::Boundary(Boundary(name, key, _, _, fallback, _)) = &*node {
        *node = ReplayNode::Element(Element(name.clone(), *key, vec![], *fallback));
    }
}

fn show(nodes: &[Shared]) -> String {
    nodes
        .iter()
        .map(|node| match &*node.borrow() {
            ReplayNode::Element(Element(_, key, children, _)) => format!("{key}:{}", children.len()),
            ReplayNode::Boundary(Boundary(_, key, _, _, _, id)) => format!("{key}@{id}"),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// Written through a `&mut`, and replaced, each the array changed in place.
fn reset(node: &mut ReplayNode) {
    *node = ReplayNode::Element(Element(None, 0.0, vec![9.0], None));
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(rust_js, rust_js::untagged)]
enum Span {
    Point((f64, f64)),
    Range((f64, f64, f64)),
}

// Cloned, not copied: a clone is apart too.
#[derive(Clone, Debug)]
#[cfg_attr(rust_js, rust_js::untagged)]
enum Cloned {
    Pair((f64, f64)),
    Triple((f64, f64, f64)),
}

fn main() {
    let element = ReplayNode::Element(Element(Some("div".into()), 1.0, vec![], None));
    let boundary = ReplayNode::Boundary(Boundary(None, 2.0, vec![3.0], Some(4.0), None, 5.0));
    // A boundary whose name is a string and whose slots are `None`, an
    // `undefined` in its array: still six items.
    let named = ReplayNode::Boundary(Boundary(Some("b".into()), 6.0, vec![], None, None, 7.0));
    for node in [&element, &boundary, &named] {
        println!(
            "{} {:?} {} {:?}",
            is_element(node),
            root_segment(node),
            key(node),
            node
        );
    }
    let copy = boundary.clone();
    println!("{} {}", copy == boundary, copy == element);

    let two = Pair::Two(Rc::new(("a".into(), 1.0)));
    let three = Pair::Three(Rc::new(("b".into(), 2.0, 3.0)));
    println!("{} {}", sum(&two), sum(&three));

    let shapes = [
        Shape::Word("word".into()),
        Shape::Point((1.0, 2.0)),
        Shape::Box((0.0, 0.0, 2.0, 3.0)),
        Shape::Items([1.0, 2.0, 4.0]),
    ];
    for shape in &shapes {
        println!("{}", area(shape));
    }

    let entries = [
        Entry::Pair(("pair".into(), 1.0)),
        Entry::Named(Rc::new(Named { name: "named".into() })),
    ];
    for entry in &entries {
        println!("{}", name(entry));
    }

    for key in [Key::Name("k".into()), Key::Id(Id(3.0))] {
        println!("{} {}", key_text(&key), matches!(key, Key::Id(_)));
    }

    let children: Vec<Shared> = (0..3)
        .map(|i| Rc::new(RefCell::new(ReplayNode::Element(Element(None, i as f64, vec![1.0; i], None)))))
        .collect();
    let mut working: HashMap<u32, Shared> = HashMap::new();
    working.insert(1, Rc::clone(&children[1]));
    track(&working[&1], Some(8.0), 10.0);
    println!("{}", show(&children));
    track(&children[1], None, 11.0);
    println!("{}", show(&children));
    untrack(&working[&1]);
    println!("{} {:?}", show(&children), children[1].borrow());
    renumber(&mut children[2].borrow_mut(), 20.0);
    renumber(&mut working[&1].borrow_mut(), 10.0);
    println!("{}", show(&children));
    track(&children[0], Some(12.0), 13.0);
    demote(&children[0]);
    println!("{} {:?}", show(&children), children[0].borrow());

    let mut node = boundary.clone();
    let held = &mut node;
    reset(held);
    println!("{node:?} {boundary:?}");
    let old = std::mem::replace(&mut node, named.clone());
    println!("{old:?} {node:?}");
    let cell = RefCell::new(element.clone());
    let before = cell.replace(boundary.clone());
    println!("{before:?} {:?}", cell.borrow());

    // A copy is apart from what it's copied of, which changes after.
    let mut span = Span::Point((1.0, 2.0));
    let copy = span;
    let place = &mut span;
    *place = Span::Range((3.0, 4.0, 5.0));
    println!("{copy:?} {span:?}");
    let mut cloned = Cloned::Pair((1.0, 2.0));
    let clone = cloned.clone();
    let place = &mut cloned;
    *place = Cloned::Triple((3.0, 4.0, 5.0));
    println!("{clone:?} {cloned:?}");
}
