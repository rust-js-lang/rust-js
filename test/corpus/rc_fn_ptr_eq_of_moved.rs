// `Rc::ptr_eq` of `Rc<dyn Fn()>`s, where one is made of a closure a `let`
// holds, neither `Copy` nor cloned, React's `subscribe(Rc::new(
// handle_store_change))`: it's given `Rc::new` once, moved, a JS function of
// its own, so the `Rc`s are the functions, uncounted (ADR 0959, amended).
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static HOOKS: RefCell<Vec<Rc<dyn Fn() -> u32>>> = const { RefCell::new(Vec::new()) };
}

fn subscribe(hook: Rc<dyn Fn() -> u32>) -> Rc<dyn Fn() -> u32> {
    HOOKS.with_borrow_mut(|hooks| hooks.push(Rc::clone(&hook)));
    hook
}

fn unsubscribe(hook: &Rc<dyn Fn() -> u32>) -> bool {
    HOOKS.with_borrow_mut(|hooks| match hooks.iter().position(|h| Rc::ptr_eq(h, hook)) {
        Some(index) => {
            hooks.remove(index);
            true
        }
        None => false,
    })
}

fn observe(count: &Rc<RefCell<u32>>) -> Rc<dyn Fn() -> u32> {
    let handle_change = {
        let count = Rc::clone(count);
        move || {
            *count.borrow_mut() += 1;
            *count.borrow()
        }
    };
    subscribe(Rc::new(handle_change))
}

fn main() {
    let count = Rc::new(RefCell::new(0));
    let a = observe(&count);
    let b = observe(&count);
    println!("{} {} {}", a(), b(), Rc::ptr_eq(&a, &b));
    println!("{} {} {}", unsubscribe(&a), unsubscribe(&a), HOOKS.with_borrow(Vec::len));
}
