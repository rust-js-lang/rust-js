// A variable shadowed by its own value in an `else`, `let value:
// &Option<u32> = value;`, is the variable it shadows (ADR 0277); a function
// declared beside it whose `match` names its `Result` apart from its own
// `value`, `value$1`, keeps its own: each read of it is of the `Result`,
// not of the function's `value` (#1766, found with tidy's hints emptied,
// where the shadow sat in an `else`, as here).
use std::cell::RefCell;

fn checked(value: &Option<u32>, id: u32) -> Result<String, String> {
    match value {
        Some(0) | None => Err("zero".to_string()),
        Some(v) => Ok(format!("{v}:{id}")),
    }
}

fn resolve(value: &Option<u32>, id: u32, log: &RefCell<Vec<String>>) {
    fn retry(id: u32, log: &RefCell<Vec<String>>) {
        let value = Some(id);
        match checked(&value, id) {
            Ok(part) => log.borrow_mut().push(part),
            Err(reason) => log.borrow_mut().push(format!("rejected {reason} {value:?}")),
        }
    }
    if id > 1 {
        retry(id - 2, log);
    } else {
        let value: &Option<u32> = value;
        log.borrow_mut().push(format!("{value:?}"));
    }
}

fn main() {
    let log = RefCell::new(Vec::new());
    resolve(&Some(5), 4, &log);
    resolve(&Some(5), 2, &log);
    resolve(&Some(7), 1, &log);
    resolve(&None, 0, &log);
    println!("{:?}", log.borrow());
}
