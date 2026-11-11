//@ library-differs
// A write's result, `fmt::Result` or `io::Result<()>`, given to generic code
// that has a `Result` of its type parameters, and given back (#1408):
// `undefined` or its error where it's a write's, `{ TAG }` in generic code,
// adapted where it crosses, as an argument, a result, a closure's result, a
// trait's generic method's argument, a generic method's result, a `fn`
// pointer's argument and a `?` of what generic code gives; and as a type
// parameter's value, its error thrown where it's given back.
use std::fmt::{self, Write as _};
use std::io::{self, Write};

struct Fails;

impl io::Write for Fails {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("refused"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn report<T: fmt::Debug>(what: &str, result: io::Result<T>) {
    match result {
        Ok(value) => println!("{what}: {value:?}"),
        Err(e) => println!("{what}: {e}"),
    }
}

fn shown<T: fmt::Debug, E: fmt::Debug>(r: &Result<T, E>) -> String {
    format!("{r:?}")
}

fn passed<T>(r: io::Result<T>) -> io::Result<T> {
    r
}

fn err_of<E>(e: E) -> Result<(), E> {
    Err(e)
}

fn retried<T, F: FnMut() -> io::Result<T>>(mut f: F, times: usize) -> io::Result<T> {
    let mut last = f();
    for _ in 1..times {
        if last.is_ok() {
            break;
        }
        last = f();
    }
    last
}

trait Check {
    fn check<T: fmt::Debug>(&self, r: io::Result<T>) -> String;
}

struct Checker;

impl Check for Checker {
    fn check<T: fmt::Debug>(&self, r: io::Result<T>) -> String {
        format!("{:?}", r.map(|t| format!("{t:?}")))
    }
}

fn through<C: Check>(c: &C, r: io::Result<()>) -> String {
    c.check(r)
}

struct Holder<T> {
    value: T,
}

impl<T> Holder<T> {
    fn ok(self) -> io::Result<T> {
        Ok(self.value)
    }
}

fn tail(fail: bool) -> io::Result<()> {
    let r = if fail { writeln!(Fails, "x") } else { Ok(()) };
    passed(r)
}

fn tried(fail: bool) -> io::Result<u8> {
    let r = if fail { writeln!(Fails, "x") } else { Ok(()) };
    passed(r)?;
    Ok(7)
}

struct Shown(bool);

impl fmt::Display for Shown {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let r = if self.0 { Err(fmt::Error) } else { write!(f, "shown") };
        err_or(r)
    }
}

fn err_or<E>(r: Result<(), E>) -> Result<(), E> {
    r
}

fn same<T>(t: T) -> T {
    t
}

fn held_back(fail: bool) -> io::Result<()> {
    let r = if fail { writeln!(Fails, "x") } else { Ok(()) };
    same(r)
}

fn after(fail: bool) -> io::Result<u8> {
    held_back(fail)?;
    Ok(1)
}

fn forward(r: io::Result<()>) -> io::Result<()> {
    same(r)
}

fn main() {
    // Given as an argument, written or failed, of a writer's or a stream's.
    let mut v = Vec::new();
    report("vec", writeln!(v, "x"));
    report("fails", writeln!(Fails, "x"));
    report("flush", io::stdout().flush());
    let mut s = String::new();
    println!("{} {s}", shown(&write!(s, "y")));
    println!("{}", shown(&Fails.write_all(b"z")));

    // Given back: made `Ok` or `Err` in generic code, a `fmt::Error` too.
    let made: io::Result<()> = passed(Ok(()));
    println!("{} {made:?}", made.is_ok());
    let failed: io::Result<()> = passed(Err(io::Error::other("no")));
    if let Err(e) = &failed {
        println!("{e} {}", failed.is_err());
    }
    let r: fmt::Result = err_of(fmt::Error);
    println!("{r:?} {}", r.is_err());
    println!("{:?} {:?}", tail(false), tail(true));
    println!("{:?} {:?}", tried(false), tried(true));

    // A closure's, called by generic code until it's `Ok`.
    let mut n = 0;
    let r = retried(
        || {
            n += 1;
            if n < 3 { writeln!(Fails, "y") } else { Ok(()) }
        },
        5,
    );
    println!("{r:?} {n}");
    let mut m = 0;
    let r = retried(
        || {
            m += 1;
            writeln!(Fails, "z")
        },
        2,
    );
    println!("{r:?} {m}");

    // A trait's generic method's, directly and through generic code; a
    // generic method's; a `fn` pointer's.
    println!("{}", Checker.check(writeln!(v, "a")));
    println!("{}", through(&Checker, writeln!(Fails, "b")));
    let held: io::Result<()> = Holder { value: () }.ok();
    println!("{held:?}");
    let f: fn(&Result<(), fmt::Error>) -> String = shown;
    println!("{} {}", f(&Err(fmt::Error)), f(&Ok(())));

    // Held by generic code as a type parameter's value, and given back.
    println!("{:?} {:?}", held_back(false), held_back(true));
    println!("{:?} {:?}", after(false), after(true));
    let kept = same(writeln!(Fails, "y"));
    println!("{}", kept.is_err());
    println!("{:?} {:?}", forward(writeln!(v, "z")), forward(writeln!(Fails, "z")).map_err(|e| e.kind()));

    // A `Display` whose result generic code passes on.
    println!("{}", Shown(false));
    let mut text = String::new();
    println!("{:?} {text:?}", fmt::write(&mut text, format_args!("{}", Shown(true))));
}
