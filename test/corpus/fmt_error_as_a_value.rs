// A write's result made as a value, `let err: fmt::Result =
// Err(fmt::Error)`, is its error, a `$FmtError`, not thrown where it's
// made, so `is_ok()` is `false`; held in a field, a `RefCell` or a
// `Some`, its `is_ok()` is asked of it, where it was `true`; given back
// by a function, it's thrown as a call of one throws, `$writeCheck(r)`;
// and a `write!` to a `String` reached through a call, `*s.as_mut()
// .unwrap()`, is caught as one to a variable is (#1011).
use std::cell::RefCell;
use std::fmt::{self, Write};

struct Hours(u32);

impl fmt::Display for Hours {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0 > 23 {
            return Err(fmt::Error);
        }
        write!(f, "{:02}h", self.0)
    }
}

struct Held(fmt::Result);

fn made(fail: bool) -> fmt::Result {
    let r = if fail { Err(fmt::Error) } else { Ok(()) };
    r
}

fn passed_on(fail: bool) -> fmt::Result {
    made(fail)?;
    Ok(())
}

fn main() {
    let err: fmt::Result = Err(fmt::Error);
    println!("{} {}", err.is_ok(), err.is_err());
    let ok: fmt::Result = Ok(());
    println!("{} {:?}", ok.is_ok(), ok);
    println!("{:?}", err);
    let held = Held(Err(fmt::Error));
    println!("{}", held.0.is_ok());
    let cell: RefCell<fmt::Result> = RefCell::new(Err(fmt::Error));
    println!("{}", cell.borrow().is_ok());
    let mut s = String::new();
    let w = write!(s, "{}", Hours(30));
    println!("{} {:?}", w.is_err(), s);
    let kept = Some(write!(s, "{}", Hours(30)));
    println!("{}", kept.as_ref().unwrap().is_ok());
    println!("{:?} {:?}", made(true), made(false));
    println!("{:?} {:?}", passed_on(true), passed_on(false));
    let results: [fmt::Result; 2] = [Ok(()), Err(fmt::Error)];
    println!("{:?}", results.iter().map(|r| r.is_ok()).collect::<Vec<_>>());
    let mut t = Some(String::from("<"));
    let again = write!(*t.as_mut().unwrap(), "{}|{}", Hours(5), Hours(40));
    println!("{:?} {:?}", again, t);
}
