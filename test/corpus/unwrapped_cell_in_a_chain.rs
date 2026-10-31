// A cell's field read through an `unwrap()` in an `&&`'s operand, React
// DOM's mayResourceSuspendCommit, `resource.type === 'stylesheet' &&
// (resource.state.loading & Inserted) === NotLoaded`: the chain as written,
// its `unwrap` where it's read, `$unwrap(resource.state).loading`, so a
// resource of another type, of no state, never unwraps it; read twice, or
// changed, what holds it is named.
use std::cell::Cell;
use std::rc::Rc;

struct State {
    loading: Cell<u32>,
}

struct Resource {
    kind: &'static str,
    state: Option<Rc<State>>,
}

const INSERTED: u32 = 4;

fn may_suspend(resource: &Resource) -> bool {
    resource.kind == "stylesheet" && (resource.state.as_ref().unwrap().loading.get() & INSERTED) == 0
}

fn inserted(resource: &Resource) -> u32 {
    let state = resource.state.as_ref().unwrap();
    state.loading.set(state.loading.get() | INSERTED);
    resource.state.as_ref().unwrap().loading.get()
}

fn main() {
    let sheet = Resource {
        kind: "stylesheet",
        state: Some(Rc::new(State { loading: Cell::new(1) })),
    };
    let script = Resource {
        kind: "script",
        state: None,
    };
    println!("{} {}", may_suspend(&sheet), may_suspend(&script));
    println!("{} {}", inserted(&sheet), may_suspend(&sheet));
}
