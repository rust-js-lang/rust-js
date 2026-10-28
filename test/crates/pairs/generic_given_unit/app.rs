//! React DOM's `runWithFiberInDEV(ancestor, () => { .. })`, given `()`s,
//! which the reconciler's export takes as they are (gap rd75b), and its
//! `runWithFiberInDEV(instance, executeDispatch, event, listener,
//! currentTarget)`, given two (gap rd85).
use std::rc::Rc;

fn execute_dispatch(event: &str, listener: &str, current_target: &str) -> String {
    format!("{event} {listener} {current_target} {:?}", dep::tag_now())
}

pub fn main() {
    let fiber = Rc::new(dep::Fiber { tag: 3 });
    let tag = "b";
    let said = dep::run_with_fiber(
        Some(Rc::clone(&fiber)),
        |_: ()| format!("<{tag}> {:?}", dep::tag_now()),
        (),
    );
    println!("{said} {:?} {}", dep::tag_now(), dep::own());
    println!("{:?} {:?}", dep::wrapped(()), dep::wrapped_own());
    dep::run_with_fiber_in_dev(
        Some(Rc::clone(&fiber)),
        |(), (), (), (), ()| println!("<{tag}> {:?}", dep::tag_now()),
        (),
        (),
        (),
        (),
        (),
    );
    let dispatched = dep::run_with_fiber_in_dev(
        Some(fiber),
        |a, b, c, (), ()| execute_dispatch(a, b, c),
        "click",
        "onClick",
        "button",
        (),
        (),
    );
    println!("{dispatched} {:?} {}", dep::tag_now(), dep::own_in_dev());
}
