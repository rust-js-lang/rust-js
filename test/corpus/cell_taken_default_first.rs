// A cell's `take` makes its `Default` first: std's is
// `self.replace(Default::default())`, so a `Default` of the crate's that
// prints, and borrows and writes the cell it's taken of, runs before the
// cell is borrowed, and what's moved out is what it wrote. Of a `RefCell`
// whose borrows are counted, one of an object, a `Cell`, and a
// thread-local's, whose initializer runs before its `Default` (#699).
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Debug)]
struct Log(u32);

#[derive(Debug)]
struct Items {
    items: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Count(u32);

thread_local! {
    static LOG: RefCell<Option<Rc<RefCell<Log>>>> = const { RefCell::new(None) };
    static ITEMS: RefCell<Option<Rc<RefCell<Items>>>> = const { RefCell::new(None) };
    static COUNT: RefCell<Option<Rc<Cell<Count>>>> = const { RefCell::new(None) };
    static LOCAL: RefCell<Log> = {
        println!("local made");
        RefCell::new(Log(3))
    };
}

impl Default for Log {
    fn default() -> Self {
        println!("log default");
        LOG.with_borrow(|log| {
            if let Some(cell) = log {
                cell.borrow_mut().0 += 10;
            }
        });
        Log(0)
    }
}

impl Default for Items {
    fn default() -> Self {
        println!("items default");
        ITEMS.with_borrow(|items| {
            if let Some(cell) = items {
                cell.borrow_mut().items.push(9);
            }
        });
        Items { items: Vec::new() }
    }
}

impl Default for Count {
    fn default() -> Self {
        println!("count default");
        COUNT.with_borrow(|count| {
            if let Some(cell) = count {
                cell.set(Count(cell.get().0 + 10));
            }
        });
        Count(0)
    }
}

fn main() {
    let log = Rc::new(RefCell::new(Log(1)));
    LOG.with_borrow_mut(|cell| *cell = Some(log.clone()));
    // Counted: a guard held across a call that may borrow it.
    let held = log.borrow();
    println!("held {:?}", *held);
    drop(held);
    let old = log.take();
    println!("took {:?}, left {:?}", old, log.borrow());

    let items = Rc::new(RefCell::new(Items { items: vec![1] }));
    ITEMS.with_borrow_mut(|cell| *cell = Some(items.clone()));
    let held = items.borrow();
    println!("held {:?}", *held);
    drop(held);
    let old = items.take();
    println!("took {:?}, left {:?}", old, items.borrow());

    let count = Rc::new(Cell::new(Count(1)));
    COUNT.with_borrow_mut(|cell| *cell = Some(count.clone()));
    let old = count.take();
    println!("took {:?}, left {:?}", old, count.get());

    let old = LOCAL.take();
    println!("took {:?}, left {:?}", old, LOCAL.with_borrow(|l| l.0));
}
