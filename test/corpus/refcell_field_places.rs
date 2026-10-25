//@ run-pass
// A struct's `RefCell` field holds its value by its declaration, as JS
// that makes the object writes it, whatever else has its type (ADR 0863,
// amended): a generic struct's `RefCell<T>`, borrowed while code that may
// borrow it runs, leaves a `RefCell<Vec<f64>>` field the array; a
// thread-local of a nullable field's type leaves the field the fiber; one
// lent, moved out or written whole is converted there, its borrows counted
// on the field's one cell.
use std::cell::RefCell;
use std::rc::Rc;

pub struct Root {
    pub times: RefCell<Vec<f64>>,
}

pub fn mark(root: &Root, index: usize) {
    let mut times = root.times.borrow_mut();
    times[index] = 1.0;
}

pub struct Cursor<T> {
    pub current: RefCell<T>,
}

pub fn read<T: Clone>(cursor: &Cursor<T>) -> T {
    cursor.current.borrow().clone()
}

pub struct Fiber {
    pub tag: u8,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub r#return: RefCell<Option<Rc<Fiber>>>,
}

thread_local! {
    pub static CURRENT: RefCell<Option<Rc<Fiber>>> = const { RefCell::new(None) };
}

pub fn set_current_fiber(fiber: Option<Rc<Fiber>>) {
    CURRENT.set(fiber);
}

pub fn current_tag() -> Option<u8> {
    CURRENT.with_borrow(|current| current.as_ref().map(|fiber| fiber.tag))
}

pub fn parent_tag(fiber: &Fiber) -> u8 {
    fiber.r#return.borrow().as_ref().map_or(0, |parent| parent.tag)
}

pub struct Wrap {
    pub n: RefCell<u16>,
}

fn bump(cell: &RefCell<u16>) -> u16 {
    *cell.borrow_mut() += 10;
    *cell.borrow()
}

fn main() {
    let root = Root {
        times: RefCell::new(vec![0.0; 3]),
    };
    mark(&root, 1);
    println!("{:?}", root.times.borrow());

    let numbers = Cursor {
        current: RefCell::new(5u8),
    };
    *numbers.current.borrow_mut() += 1;
    let names = Cursor {
        current: RefCell::new(String::from("root")),
    };
    names.current.borrow_mut().push('!');
    println!("{} {}", read(&numbers), read(&names));

    let parent = Rc::new(Fiber {
        tag: 3,
        r#return: RefCell::new(None),
    });
    let child = Fiber {
        tag: 5,
        r#return: RefCell::new(Some(Rc::clone(&parent))),
    };
    println!("{} {}", parent_tag(&child), parent_tag(&parent));
    set_current_fiber(Some(Rc::clone(&parent)));
    println!("{:?}", current_tag());
    set_current_fiber(None);
    println!("{:?}", current_tag());
    CURRENT.with_borrow_mut(|current| *current = Some(Rc::clone(&parent)));
    println!("{:?}", current_tag());
    child.r#return.replace(None);
    println!("{}", parent_tag(&child));

    let mut wrap = Wrap { n: RefCell::new(5) };
    *wrap.n.borrow_mut() += 1;
    println!("{}", bump(&wrap.n));
    let Wrap { n } = wrap;
    wrap.n = RefCell::new(9);
    *n.borrow_mut() += 1;
    println!("{} {}", n.borrow(), wrap.n.borrow());
}
