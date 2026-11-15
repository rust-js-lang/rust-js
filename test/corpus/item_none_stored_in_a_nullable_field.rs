// A plain `const`'s or thread-local's `None` is `undefined` in JS, so a
// nullable field stores it `?? null`, as Rust's `None`: its read isn't one
// of a function, which is never `undefined` (#1957, React's
// ReactFiberNewContext `rendererSigil`).
use std::cell::Cell;

thread_local! {
    static SIGIL: Option<&'static str> = None;
}

const NONE_YET: Option<&str> = None;
const FIRST: Option<&str> = Some("first");

struct Context {
    #[cfg_attr(rust_js, rust_js::nullable)]
    current: Cell<Option<&'static str>>,
}

fn pushed_sigil(context: &Context) {
    let sigil = SIGIL.with(|&sigil| sigil);
    context.current.set(sigil);
}

fn pushed(context: &Context, value: Option<&'static str>) {
    context.current.set(value);
}

fn main() {
    let context = Context {
        current: Cell::new(Some("start")),
    };
    pushed_sigil(&context);
    println!("{} {:?}", context.current.get().is_some(), context.current.get());
    pushed(&context, FIRST);
    println!("{} {:?}", context.current.get().is_some(), context.current.get());
    context.current.set(NONE_YET);
    println!("{} {:?}", context.current.get().is_some(), context.current.get());
}
