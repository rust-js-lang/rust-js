// A variable shadowed by its own value, `let value: &Option<u32> =
// value;`, is the variable it shadows (ADR 0277); a closure's own of the
// name that's given apart as, `value$1`, its `catch_unwind`'s result,
// named apart from the `value` it captures, stays its own: each of its
// reads is of the result, never the captured `value` (#457, React's
// ReactFlightReplyClient's lazy case's `retry`). So too a function
// declared in it, whose `Result` is named apart from its own `value`.
use std::cell::RefCell;

fn serialize(value: &Option<u32>, id: u32) -> String {
    match value {
        Some(0) | None => panic!("zero"),
        Some(v) => format!("{v}:{id}"),
    }
}

fn checked(value: &Option<u32>, id: u32) -> Result<String, String> {
    match value {
        Some(0) | None => Err("zero".to_string()),
        Some(v) => Ok(format!("{v}:{id}")),
    }
}

fn thrown(reason: &(dyn std::any::Any + Send)) -> String {
    reason.downcast_ref::<&str>().map_or("?".to_string(), |s| s.to_string())
}

fn then(on_fulfilled: &dyn Fn(), on_rejected: &dyn Fn()) {
    on_fulfilled();
    on_rejected();
}

fn resolve(value: &'static Option<u32>, id: u32, log: &'static RefCell<Vec<String>>) {
    if id > 1 {
        let retry = move || match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| serialize(value, id))) {
            Ok(part) => log.borrow_mut().push(part),
            Err(reason) => log.borrow_mut().push(format!("rejected {}", thrown(&*reason))),
        };
        then(&retry, &retry);
        return;
    }
    let value: &Option<u32> = value;
    log.borrow_mut().push(format!("{value:?}"));
}

fn resolve_declared(value: &'static Option<u32>, id: u32, log: &RefCell<Vec<String>>) {
    fn retry(id: u32, log: &RefCell<Vec<String>>) {
        let value = Some(id);
        match checked(&value, id) {
            Ok(part) => log.borrow_mut().push(part),
            Err(reason) => log.borrow_mut().push(format!("rejected {reason} {value:?}")),
        };
    }
    if id > 1 {
        retry(id - 2, log);
        return;
    }
    let value: &Option<u32> = value;
    log.borrow_mut().push(format!("{value:?}"));
}

fn main() {
    let log: &'static RefCell<Vec<String>> = Box::leak(Box::new(RefCell::new(Vec::new())));
    resolve(Box::leak(Box::new(Some(5))), 2, log);
    resolve(Box::leak(Box::new(Some(0))), 3, log);
    resolve(Box::leak(Box::new(Some(7))), 1, log);
    resolve_declared(&Some(5), 4, log);
    resolve_declared(&Some(5), 2, log);
    resolve_declared(&Some(7), 1, log);
    println!("{:?}", log.borrow());
}
