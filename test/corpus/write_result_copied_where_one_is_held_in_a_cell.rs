// A copy of a write's result, `*r.as_ref().unwrap()`, in a crate that
// holds one where it may change, a `RefCell<fmt::Result>`: itself, its
// `Ok(())` and its `Err`, never a copy of a tagged object (#1757).
use std::cell::RefCell;
use std::fmt;

fn made(fail: bool) -> fmt::Result {
    let r = Some(if fail { Err(fmt::Error) } else { Ok(()) });
    *r.as_ref().unwrap()
}

fn passed_on(fail: bool) -> fmt::Result {
    made(fail)?;
    Ok(())
}

fn main() {
    let cell: Option<RefCell<fmt::Result>> = Some(RefCell::new(Err(fmt::Error)));
    println!("{:?} {:?}", made(true), made(false));
    println!("{:?} {:?}", passed_on(true), passed_on(false));
    *cell.as_ref().unwrap().borrow_mut() = Ok(());
    println!("{:?}", cell.as_ref().unwrap().borrow());
}
