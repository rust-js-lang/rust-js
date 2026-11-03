// A scope struct may be of type parameters (ADR 1043, amended, #500): each
// call of its function makes its one value, of its own types, so its methods
// are functions declared in the function, as of a struct of none, each the
// lowering its function's calls of it need. As alien-signals'
// `createReactiveSystem` is generic over its node's kind, `struct
// System<K>` of its callbacks, and gives back closures that pass on to its
// methods, which are those methods' functions.
use std::cell::{Cell, RefCell};
use std::fmt::Display;
use std::marker::PhantomData;
use std::panic::Location;
use std::rc::Rc;

pub struct Node<K> {
    pub flags: Cell<i32>,
    pub kind: K,
}

pub struct Options<K> {
    pub notify: Box<dyn Fn(&Rc<Node<K>>)>,
}

pub struct ReactiveSystem<K> {
    pub propagate: Rc<dyn Fn(&Rc<Node<K>>)>,
    pub shallow_propagate: Rc<dyn Fn(&Rc<Node<K>>)>,
}

struct System<K> {
    notify: Box<dyn Fn(&Rc<Node<K>>)>,
}

pub fn create_reactive_system<K: 'static>(Options { notify }: Options<K>) -> ReactiveSystem<K> {
    let system = Rc::new(System { notify });
    ReactiveSystem {
        propagate: {
            let system = Rc::clone(&system);
            Rc::new(move |node| system.propagate(node))
        },
        shallow_propagate: Rc::new(move |node| system.shallow_propagate(node)),
    }
}

impl<K> System<K> {
    fn propagate(&self, node: &Rc<Node<K>>) {
        node.flags.set(node.flags.get() | 32);
        self.shallow_propagate(node);
    }

    fn shallow_propagate(&self, node: &Rc<Node<K>>) {
        if node.flags.get() & 32 != 0 {
            (self.notify)(node);
        }
    }
}

// JS calls it, whose `T` may be anything, `undefined` too; the crate's
// call of one of an `Option` calls a copy whose `Some` is a box, and each
// declares the lowering of the methods it calls.
struct Kept<T> {
    count: Cell<usize>,
    last: RefCell<Option<T>>,
}

impl<T> Kept<T> {
    fn add(&self, item: T) {
        self.count.set(self.count.get() + 1);
        self.put(Some(item));
    }

    fn put(&self, item: Option<T>) {
        *self.last.borrow_mut() = item;
    }

    fn last_is_some(&self) -> bool {
        self.last.borrow().is_some()
    }
}

pub fn keep_last<T>(items: Vec<T>, cleared: bool) -> (usize, bool, Option<T>) {
    let kept = Kept {
        count: Cell::new(0),
        last: RefCell::new(None),
    };
    for item in items {
        kept.add(item);
    }
    if cleared {
        kept.put(None);
    }
    (kept.count.get(), kept.last_is_some(), kept.last.take())
}

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

// A method given a drop of its type parameter's is given it by each call:
// the struct stays an object of methods (ADR 0047), in a generic function
// and of a concrete type alike.
struct Stack<K> {
    items: RefCell<Vec<K>>,
}

impl<K> Stack<K> {
    fn push(&self, k: K) {
        self.items.borrow_mut().push(k);
    }

    fn pop(&self) {
        let popped = self.items.borrow_mut().pop();
        drop(popped);
        println!("popped");
    }
}

fn pop_twice<T>(items: Vec<T>, more: T) -> usize {
    let stack = Stack {
        items: RefCell::new(items),
    };
    stack.push(more);
    stack.pop();
    stack.pop();
    stack.items.borrow().len()
}

struct Pile<K> {
    items: RefCell<Vec<K>>,
}

impl<K> Pile<K> {
    fn push(&self, k: K) {
        self.items.borrow_mut().push(k);
    }

    fn pop(&self) {
        let popped = self.items.borrow_mut().pop();
        drop(popped);
        println!("popped");
    }
}

fn pop_loud() -> usize {
    let pile = Pile {
        items: RefCell::new(vec![Loud("x")]),
    };
    pile.push(Loud("y"));
    pile.pop();
    pile.items.borrow().len()
}

// And one given a bound's dictionary.
struct Shown<T> {
    lines: RefCell<Vec<String>>,
    last: RefCell<Option<T>>,
}

impl<T: Display + Clone> Shown<T> {
    fn show(&self, item: &T) {
        self.lines.borrow_mut().push(format!("<{item}>"));
        *self.last.borrow_mut() = Some(item.clone());
    }

    fn joined(&self) -> String {
        self.lines.borrow().join(" ")
    }
}

fn show_all<T: Display + Clone>(items: &[T]) -> String {
    let shown = Shown {
        lines: RefCell::new(Vec::new()),
        last: RefCell::new(None),
    };
    for item in items {
        shown.show(item);
    }
    shown.joined()
}

// And one given a fact of its type parameter, its size.
struct Sizes<K> {
    count: Cell<usize>,
    kind: PhantomData<K>,
}

impl<K> Sizes<K> {
    fn bytes(&self) -> usize {
        self.count.get() * std::mem::size_of::<K>()
    }
}

fn bytes_of<K>(n: usize) -> usize {
    let sizes = Sizes::<K> {
        count: Cell::new(n),
        kind: PhantomData,
    };
    sizes.bytes()
}

// A method given its caller's location is given it by each call, of a
// struct of no type parameters too (#529).
struct Lines {
    first: Cell<u32>,
}

impl Lines {
    #[track_caller]
    fn line(&self) -> u32 {
        let line = Location::caller().line();
        if self.first.get() == 0 {
            self.first.set(line);
        }
        line - self.first.get()
    }
}

fn lines() -> (u32, u32) {
    let lines = Lines { first: Cell::new(0) };
    let a = lines.line();
    let b = lines.line();
    (a, b)
}

// An impl of other arguments than its own parameters, `impl Tally<u8>`,
// whose methods' arguments aren't the literal's, stays the object.
struct Tally<T> {
    items: RefCell<Vec<T>>,
}

impl Tally<u8> {
    fn total(&self) -> u32 {
        self.items.borrow().iter().map(|&n| u32::from(n)).sum()
    }
}

fn tally(items: Vec<u8>) -> u32 {
    let tally = Tally {
        items: RefCell::new(items),
    };
    tally.total()
}

fn main() {
    let seen = Rc::new(Cell::new(0));
    let counted = Rc::clone(&seen);
    let system = create_reactive_system(Options {
        notify: Box::new(move |node: &Rc<Node<String>>| {
            counted.set(counted.get() + node.kind.len() as i32);
        }),
    });
    let other = create_reactive_system(Options {
        notify: Box::new(|node: &Rc<Node<u8>>| println!("other {}", node.kind)),
    });
    let node = Rc::new(Node {
        flags: Cell::new(0),
        kind: "abc".to_string(),
    });
    (system.shallow_propagate)(&node);
    (system.propagate)(&node);
    (other.propagate)(&Rc::new(Node {
        flags: Cell::new(0),
        kind: 7,
    }));
    (system.shallow_propagate)(&node);
    println!("{} {}", seen.get(), node.flags.get());

    println!("{:?}", keep_last(vec![1, 2, 3], false));
    println!("{:?}", keep_last(vec![Some(1), None], false));
    println!("{:?}", keep_last(vec![Some(1), Some(2)], true));
    println!("{:?}", keep_last(Vec::<Option<u8>>::new(), false));

    println!("{}", pop_twice(vec![Loud("a"), Loud("b")], Loud("c")));
    println!("{}", pop_twice(vec![Some(1), None], None::<u32>));
    println!("{}", pop_loud());

    println!("{}", show_all(&["x", "y"]));
    println!("{}", show_all(&[1, -2]));
    println!("{} {}", bytes_of::<u64>(3), bytes_of::<u8>(3));
    println!("{:?}", lines());
    println!("{}", tally(vec![200, 100]));
    println!("end");
}
