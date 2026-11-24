// A `RefCell` guard the source never names is named after the field it
// borrows, `log` of `request.log.borrow_mut()`, as hand-written JS names it
// and rust-analyzer names a method by its receiver, where its call's name,
// `borrowMut`, says nothing of it (#1715). A cell given as a parameter
// keeps its call's name: the parameter's own is in scope, and taken; and a
// tuple struct's, whose field `0` is no JS name, too.
use std::cell::RefCell;

struct Request {
    log: RefCell<Vec<String>>,
    seen: RefCell<u32>,
}

fn record(request: &Request, names: &[&str], check: impl Fn() -> bool) {
    for name in names {
        request.log.borrow_mut().push(format!("{name} {}", check()));
    }
    *request.seen.borrow_mut() += names.len() as u32;
}

struct Pair(RefCell<Vec<u8>>);

fn bump(pair: &Pair, check: impl Fn() -> bool) -> usize {
    pair.0.borrow_mut().push(check() as u8);
    pair.0.borrow().len()
}

fn count(cell: &RefCell<Vec<String>>, check: impl Fn() -> bool) -> usize {
    cell.borrow_mut().push(format!("counted {}", check()));
    cell.borrow().len()
}

fn main() {
    let request = Request {
        log: RefCell::new(Vec::new()),
        seen: RefCell::new(0),
    };
    record(&request, &["a", "b"], || request.seen.try_borrow_mut().is_ok());
    println!("{:?} {}", request.log.borrow(), request.seen.borrow());
    let n = count(&request.log, || request.log.try_borrow().is_err());
    println!("{n} {:?}", request.log.borrow().last());
    let pair = Pair(RefCell::new(vec![7]));
    println!("{}", bump(&pair, || pair.0.try_borrow_mut().is_err()));
}
