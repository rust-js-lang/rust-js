//@ run-fail: removal index (is 1) should be < len (is 1)
// Where `position` finds an item of a slice, its `Option<usize>` only
// tested and its index read (rd316): JS's `indexOf` of an item found by
// `===`, `findIndex` of another test, `-1` where it's `None`, as JS's give
// it; and the index found, while nothing changed the array, in bounds, so
// `remove` and `insert` of it are `splice`. React's test renderer edits a
// parent's children so.
use std::cell::RefCell;
use std::rc::Rc;

struct Node {
    name: String,
}

struct Parent {
    children: RefCell<Vec<Rc<Node>>>,
}

fn names(parent: &Parent) -> String {
    let children = parent.children.borrow();
    children.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join(",")
}

// `const index = children.indexOf(child); if (index !== -1) { children.splice(index, 1); }`
fn append_child(parent: &Parent, child: Rc<Node>) {
    let mut children = parent.children.borrow_mut();
    if let Some(index) = children.iter().position(|c| Rc::ptr_eq(c, &child)) {
        children.remove(index);
    }
    children.push(child);
}

fn insert_before(parent: &Parent, child: Rc<Node>, before_child: &Rc<Node>) {
    let mut children = parent.children.borrow_mut();
    let index = children.iter().position(|c| Rc::ptr_eq(c, before_child));
    if let Some(index) = index {
        children.insert(index, child);
    }
}

fn remove_child(parent: &Parent, child: &Rc<Node>) {
    let mut children = parent.children.borrow_mut();
    match children.iter().position(|c| Rc::ptr_eq(c, child)) {
        Some(index) => {
            children.remove(index);
        }
        None => println!("no {}", child.name),
    }
}

// `let .. else`'s: the index found in bounds too.
fn remove_first_named(parent: &Parent, name: &str) {
    let mut children = parent.children.borrow_mut();
    let Some(index) = children.iter().position(|c| c.name == name) else {
        return;
    };
    children.remove(index);
}

// A test of another kind: `findIndex`.
fn index_named(parent: &Parent, name: &str) -> String {
    let children = parent.children.borrow();
    match children.iter().position(|c| c.name == name) {
        Some(index) => format!("{name} at {index}"),
        None => format!("no {name}"),
    }
}

// Of numbers, by `==`: `indexOf`; NaN found by neither.
fn first_of(xs: &[f64], x: f64) -> String {
    if let Some(i) = xs.iter().position(|&y| y == x) {
        return format!("{x} at {i}");
    }
    format!("no {x}")
}

// Of an iterator stepped first, where what's left is: no array's search.
fn after_first(xs: &[u32], x: u32) -> String {
    let mut it = xs.iter();
    it.next();
    match it.position(|&y| y == x) {
        Some(i) => format!("{x} at {i} after the first"),
        None => format!("no {x} after the first"),
    }
}

// Of `iter_mut()`'s items, each a place, the array of them searched.
fn bump(xs: &mut [u32], x: u32) {
    if let Some(i) = xs.iter_mut().position(|y| *y == x) {
        xs[i] += 10;
    }
}

// An `Option` that leaves the function is `undefined` where it's `None`.
fn found(xs: &[u32], x: u32) -> Option<usize> {
    xs.iter().position(|&y| y == x)
}

// The array changed between: the index isn't known in bounds, and
// `remove` of one past its end panics.
fn remove_after_pop(xs: &mut Vec<u32>, x: u32) {
    if let Some(i) = xs.iter().position(|&y| y == x) {
        xs.pop();
        xs.remove(i);
        println!("{xs:?}");
    }
}

fn main() {
    let parent = Parent {
        children: RefCell::new(Vec::new()),
    };
    let a = Rc::new(Node { name: "a".to_string() });
    let b = Rc::new(Node { name: "b".to_string() });
    let c = Rc::new(Node { name: "c".to_string() });
    append_child(&parent, Rc::clone(&a));
    append_child(&parent, Rc::clone(&b));
    append_child(&parent, Rc::clone(&a));
    println!("{}", names(&parent));
    insert_before(&parent, Rc::clone(&c), &a);
    println!("{}", names(&parent));
    // Another node of the same name isn't the same node.
    let other_a = Rc::new(Node { name: "a".to_string() });
    insert_before(&parent, Rc::clone(&c), &other_a);
    println!("{}", names(&parent));
    remove_child(&parent, &b);
    remove_child(&parent, &b);
    println!("{}", names(&parent));
    println!("{}", index_named(&parent, "a"));
    println!("{}", index_named(&parent, "b"));
    println!("{}", first_of(&[1.0, 2.0, f64::NAN], 2.0));
    println!("{}", first_of(&[1.0, 2.0, f64::NAN], f64::NAN));
    println!("{:?} {:?}", found(&[1, 2, 3], 3), found(&[1, 2, 3], 4));
    println!("{} {}", after_first(&[3, 1, 3], 3), after_first(&[3, 1], 3));
    let mut ys = [1, 2, 3];
    bump(&mut ys, 2);
    bump(&mut ys, 9);
    println!("{ys:?}");
    remove_first_named(&parent, "c");
    remove_first_named(&parent, "z");
    println!("{}", names(&parent));
    let mut xs = vec![1, 2, 3, 4];
    remove_after_pop(&mut xs, 2);
    remove_after_pop(&mut xs, 9);
    remove_after_pop(&mut xs, 3);
}
