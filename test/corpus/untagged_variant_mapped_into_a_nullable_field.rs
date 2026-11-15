// A map by an untagged enum's variant is the option itself, its payload:
// what a nullable cell field held, cloned and mapped, is stored as it is in
// a nullable field, React's Flight reply server's
// `array_root.map(InitializedReason::ArrayRoot)`; a parameter's, which may
// be `undefined`, `?? null` (#1933 step 4 (E)).
use std::cell::RefCell;
use std::rc::Rc;

struct Context {
    count: u32,
}

#[cfg_attr(rust_js, rust_js::untagged)]
enum Reason {
    Root(Rc<Context>),
    Text(String),
}

enum State {
    Pending,
    Fulfilled {
        value: u32,
        #[cfg_attr(rust_js, rust_js::nullable)]
        reason: Option<Reason>,
    },
}

struct Handler {
    #[cfg_attr(rust_js, rust_js::nullable)]
    root: RefCell<Option<Rc<Context>>>,
    value: u32,
}

fn resolve(handler: &Handler, state: &RefCell<State>) {
    let root = handler.root.borrow().clone();
    *state.borrow_mut() = State::Fulfilled {
        value: handler.value,
        reason: root.map(Reason::Root),
    };
}

fn given(root: Option<Rc<Context>>, state: &RefCell<State>) {
    *state.borrow_mut() = State::Fulfilled {
        value: 2,
        reason: root.map(Reason::Root),
    };
}

fn shown(state: &RefCell<State>) -> String {
    match &*state.borrow() {
        State::Pending => "pending".to_string(),
        State::Fulfilled { value, reason: None } => format!("{value} none"),
        State::Fulfilled {
            value,
            reason: Some(Reason::Root(context)),
        } => format!("{value} root {}", context.count),
        State::Fulfilled {
            value,
            reason: Some(Reason::Text(text)),
        } => format!("{value} text {text}"),
    }
}

fn main() {
    let state = RefCell::new(State::Pending);
    println!("{}", shown(&state));
    let context = Rc::new(Context { count: 3 });
    for root in [None, Some(Rc::clone(&context))] {
        let handler = Handler {
            root: RefCell::new(root.clone()),
            value: 1,
        };
        resolve(&handler, &state);
        println!("{}", shown(&state));
        given(root, &state);
        println!("{}", shown(&state));
    }
    *state.borrow_mut() = State::Fulfilled {
        value: 4,
        reason: Some(Reason::Text("done".to_string())),
    };
    println!("{}", shown(&state));
}
