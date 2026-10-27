// `Rc::ptr_eq` of `Rc<dyn Fn()>`s each made of a closure where its `Rc` is
// made (ADR 0959): the function is a JS object made there, so `===` of the
// two functions is the identity of the two `Rc`s, and nothing is counted.
// React's test selectors keep commit hooks so, and remove one by identity.
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static COMMIT_HOOKS: RefCell<Vec<Rc<dyn Fn()>>> = const { RefCell::new(Vec::new()) };
}

fn on_commit_root() {
    COMMIT_HOOKS.with_borrow(|commit_hooks| commit_hooks.iter().for_each(|commit_hook| commit_hook()));
}

fn observe(n: u32) -> Box<dyn Fn()> {
    let commit_hook: Rc<dyn Fn()> = Rc::new(move || println!("hook {n}"));
    COMMIT_HOOKS.with_borrow_mut(|commit_hooks| commit_hooks.push(Rc::clone(&commit_hook)));
    Box::new(move || {
        COMMIT_HOOKS.with_borrow_mut(|commit_hooks| {
            let index = commit_hooks.iter().position(|hook| Rc::ptr_eq(hook, &commit_hook));
            if let Some(index) = index {
                commit_hooks.remove(index);
            }
        });
    })
}

fn one() -> u32 {
    1
}

fn main() {
    // Two hooks of one closure, each its own `Rc`.
    let first = observe(1);
    let second = observe(2);
    let third = observe(1);
    on_commit_root();
    second();
    on_commit_root();
    first();
    third();
    third();
    on_commit_root();
    println!("{}", COMMIT_HOOKS.with_borrow(Vec::len));

    // Made in a loop, each its own, though the closure is `Copy`.
    let hooks: Vec<Rc<dyn Fn() -> u32>> = (0..2u32).map(|i| Rc::new(move || i) as Rc<dyn Fn() -> u32>).collect();
    let again = Rc::clone(&hooks[0]);
    println!(
        "{} {} {}",
        Rc::ptr_eq(&hooks[0], &hooks[1]),
        Rc::ptr_eq(&hooks[0], &again),
        again() + hooks[1]()
    );

    // A closure that only calls a function is a function of its own.
    let a: Rc<dyn Fn() -> u32> = Rc::new(|| one());
    let b: Rc<dyn Fn() -> u32> = Rc::new(|| one());
    println!("{} {}", Rc::ptr_eq(&a, &b), a() + b());
}
