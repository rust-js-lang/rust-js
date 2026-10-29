// The returns a block's branches wrote, which MIR joins into one past a
// block of its own (rd98): each where it's written, with the comment or
// the blank line written before it, React DOM's
// attemptExplicitHydrationTarget (rd108).
use std::cell::Cell;
use std::rc::Rc;

struct Fiber {
    tag: Cell<i32>,
}

struct Target {
    blocked_on: Cell<Option<i32>>,
    priority: i32,
}

fn nearest(n: i32) -> Option<Rc<Fiber>> {
    (n > 0).then(|| Rc::new(Fiber { tag: Cell::new(n % 4) }))
}

fn instance_of(f: &Fiber) -> Option<i32> {
    (f.tag.get() > 0).then_some(f.tag.get() * 10)
}

fn at_priority(p: i32, f: impl FnOnce()) {
    println!("at {p}");
    f();
}

fn attempt(target: &Target, n: i32) {
    if let Some(nearest) = nearest(n) {
        let tag = nearest.tag.get();
        if tag == 1 {
            let instance = instance_of(&nearest);
            if instance.is_some() {
                // Blocked.
                target.blocked_on.set(instance);
                at_priority(target.priority, || {
                    nearest.tag.set(0);
                });

                return;
            }
        } else if tag == 3 {
            if n > 5 {
                target.blocked_on.set(Some(n));
                // We don't currently have a way to increase the priority of
                // a root other than sync.
                return;
            }
        }
    }
    target.blocked_on.set(None);
}

fn main() {
    for n in [0, 1, 3, 5, 7, 9, 11] {
        let target = Target {
            blocked_on: Cell::new(Some(-1)),
            priority: n * 2,
        };
        attempt(&target, n);
        println!("{n}: {:?}", target.blocked_on.get());
    }
}
