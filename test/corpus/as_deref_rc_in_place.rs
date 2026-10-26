// `as_deref()` of an `Option` of an `Rc` that isn't counted, which is what
// it points at: the option as it is, read where it's read, with no `const`
// of it, so a test of it in a thread-local's borrow is a plain `&&`, as
// React's `popProvider` checks the fiber it pops.
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

struct Fiber {
    tag: u32,
}

impl PartialEq for Fiber {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for Fiber {}

impl Hash for Fiber {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(self, state);
    }
}

thread_local! {
    static FIBER_STACK: RefCell<Vec<Option<Rc<Fiber>>>> = const { RefCell::new(Vec::new()) };
}

fn popped_wrong(dev: bool, index: usize, fiber: &Fiber) -> bool {
    dev && FIBER_STACK.with_borrow(|fiber_stack| fiber_stack[index].as_deref() != Some(fiber))
}

fn is_at(stack: &[Option<Rc<Fiber>>], index: usize, fiber: &Fiber) -> bool {
    stack[index].as_deref() == Some(fiber)
}

fn main() {
    let first = Rc::new(Fiber { tag: 1 });
    let second = Rc::new(Fiber { tag: 2 });
    FIBER_STACK.with_borrow_mut(|fiber_stack| {
        fiber_stack.push(Some(Rc::clone(&first)));
        fiber_stack.push(None);
    });
    println!(
        "{} {} {} {}",
        popped_wrong(true, 0, &first),
        popped_wrong(true, 0, &second),
        popped_wrong(true, 1, &first),
        popped_wrong(false, 0, &second),
    );
    let stack = vec![Some(Rc::clone(&second)), None];
    println!(
        "{} {} {} {}",
        is_at(&stack, 0, &second),
        is_at(&stack, 0, &first),
        is_at(&stack, 1, &first),
        first.tag + second.tag,
    );
}
