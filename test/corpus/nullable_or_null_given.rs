// #689, React's createCapturedValueFromError, `stack: stack`, whose one
// call gives `componentStack ?? null`: `a.or(none())` is `a` where it's
// `Some`, else `null`, so a nullable parameter given it is `null` or a
// value, whatever `a`'s `None` is, `find`'s `undefined` here; so is
// `a.or_else(none)`, and a nullable field's `take()` and `replace(v)`.
// The callee stores each as it is, in a nullable field of a struct only
// the crate makes, tested `=== null`. One given `a.or(b)` of a `b` that
// may be `undefined`, or `a.and(none())`, whose `None` is
// `undefined`, keeps `?? null`. Native and JS agree.
struct CapturedValue {
    value: u32,
    #[cfg_attr(rust_js, rust_js::nullable)]
    stack: Option<&'static str>,
}

struct Root {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pending: Option<&'static str>,
}

#[cfg_attr(rust_js, rust_js::nullable(stack))]
fn create_captured_value_from_error(value: u32, stack: Option<&'static str>) -> CapturedValue {
    CapturedValue { value, stack }
}

#[cfg_attr(rust_js, rust_js::nullable(stack))]
fn create_captured_else(value: u32, stack: Option<&'static str>) -> CapturedValue {
    CapturedValue { value, stack }
}

#[cfg_attr(rust_js, rust_js::nullable(stack))]
fn create_captured_taken(value: u32, stack: Option<&'static str>) -> CapturedValue {
    CapturedValue { value, stack }
}

#[cfg_attr(rust_js, rust_js::nullable(stack))]
fn create_captured_found(value: u32, stack: Option<&'static str>) -> CapturedValue {
    CapturedValue { value, stack }
}

#[cfg_attr(rust_js, rust_js::nullable(stack))]
fn create_captured_and(value: u32, stack: Option<&'static str>) -> CapturedValue {
    CapturedValue { value, stack }
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn none() -> Option<&'static str> {
    None
}

fn shown(captured: &CapturedValue) -> String {
    match captured.stack {
        Some(stack) => format!("{} {stack}", captured.value),
        None => format!("{} none {}", captured.value, captured.stack.is_none()),
    }
}

fn find(wanted: &str) -> Option<&'static str> {
    ["a", "bc"].iter().copied().find(|w| *w == wanted)
}

fn main() {
    for wanted in ["z", "a", "bc"] {
        let component_stack = find(wanted);
        let captured = create_captured_value_from_error(1, component_stack.or(none()));
        println!("{}", shown(&captured));
        println!("{}", shown(&create_captured_else(2, component_stack.or_else(none))));
        let mut root = Root { pending: None };
        if wanted == "a" {
            root.pending = component_stack;
        }
        let replaced = root.pending.replace("r");
        let taken = create_captured_taken(3, replaced.or(root.pending.take()));
        println!("{} {}", shown(&taken), root.pending.is_none());
        println!("{}", shown(&create_captured_found(4, none().or(find(wanted)))));
        println!("{}", shown(&create_captured_and(5, component_stack.and(none()))));
    }
}
