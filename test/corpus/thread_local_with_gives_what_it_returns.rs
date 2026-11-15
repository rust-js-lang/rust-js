// React's popProvider: what a nullable thread-local cursor holds, read
// through `with(|cursor| *cursor.current.borrow())`, `null` or a value,
// restored to a context's nullable field after the cursor is popped, as it
// is (#1933). Each restore of a `None` and of a value.
use std::cell::{Cell, RefCell};

struct StackCursor<T> {
    #[cfg_attr(rust_js, rust_js::nullable)]
    current: RefCell<T>,
    stack: RefCell<Vec<T>>,
}

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static CURSOR: StackCursor<Option<&'static str>> = const {
        StackCursor {
            current: RefCell::new(None),
            stack: RefCell::new(Vec::new()),
        }
    };
}

struct Context {
    #[cfg_attr(rust_js, rust_js::nullable)]
    current_renderer: Cell<Option<&'static str>>,
}

fn push(context: &Context, renderer: &'static str) {
    CURSOR.with(|cursor| {
        let previous = cursor.current.replace(context.current_renderer.get());
        cursor.stack.borrow_mut().push(previous);
    });
    context.current_renderer.set(Some(renderer));
}

fn pop(context: &Context) {
    let current = CURSOR.with(|cursor| *cursor.current.borrow());
    CURSOR.with(|cursor| {
        let previous = cursor.stack.borrow_mut().pop().flatten();
        cursor.current.replace(previous);
    });
    context.current_renderer.set(current);
}

fn main() {
    let context = Context {
        current_renderer: Cell::new(None),
    };
    push(&context, "primary");
    push(&context, "secondary");
    println!("{:?}", context.current_renderer.get());
    pop(&context);
    println!("{:?}", context.current_renderer.get());
    pop(&context);
    println!("{:?}", context.current_renderer.get());
}
