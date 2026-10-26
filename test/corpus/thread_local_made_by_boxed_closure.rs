// React's ReactChildFiber: `export const reconcileChildFibers =
// createChildReconciler(true);`, a factory of a closure that captures its
// flag. Its maker only makes a value, a boxed closure, which `Box::new` is
// (ADR 0069), so the thread-local is made as its module loads, the value
// itself, and a call of it is a call of the closure (ADR 0037, amended).
use std::cell::Cell;

thread_local! {
    static CALLS: Cell<u32> = const { Cell::new(0) };
}

pub type Reconciler = dyn Fn(u32) -> u32;

fn create_child_reconciler(should_track_side_effects: bool) -> Box<Reconciler> {
    Box::new(move |lanes| {
        CALLS.set(CALLS.get() + 1);
        if should_track_side_effects { lanes | 2 } else { lanes }
    })
}

thread_local! {
    pub static RECONCILE_CHILD_FIBERS: Box<Reconciler> = create_child_reconciler(true);
    pub static MOUNT_CHILD_FIBERS: Box<Reconciler> = create_child_reconciler(false);
}

fn main() {
    let reconciled = RECONCILE_CHILD_FIBERS.with(|reconcile| reconcile(1));
    let mounted = MOUNT_CHILD_FIBERS.with(|mount| mount(1));
    let again = RECONCILE_CHILD_FIBERS.with(|reconcile| reconcile(4));
    println!("{reconciled} {mounted} {again} {}", CALLS.get());
}
