// A `format_args!` given on, to a function or a struct, or made in a loop, is
// formatted where it's written, as often as it's written: its operands are
// evaluated where it's made, in order, and only their `Display` and `Debug`
// run when it's formatted, in the callee, after what the callee did first
// (#1565). So a writer's print lands after the callee's `said: `, one
// written twice prints twice, one never written never prints, an operand's
// own effect happens once, where the macro is written, and a `Cell` set
// between is shown as it is when it's written.
use std::cell::Cell;
use std::fmt::{self, Write};
use std::panic::catch_unwind;

struct Noisy(u32);

impl fmt::Debug for Noisy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        print!("<noisy {}>", self.0);
        write!(f, "N{}", self.0)
    }
}

struct Shows<'a>(&'a Cell<u32>);

impl fmt::Display for Shows<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cell {}", self.0.get())
    }
}

struct Panics;

impl fmt::Debug for Panics {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        panic!("formatted");
    }
}

struct Bad;

impl fmt::Debug for Bad {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("B")?;
        Err(fmt::Error)
    }
}

fn said(what: fmt::Arguments) {
    print!("said: ");
    let text = what.to_string();
    println!("{text}");
}

fn twice(what: fmt::Arguments) {
    println!("twice: {what} and {what}");
}

fn never(_what: fmt::Arguments) {
    println!("never formatted");
}

fn passed_on(what: fmt::Arguments) {
    print!("passed on, ");
    said(what);
}

fn put(out: &mut String, what: fmt::Arguments) -> fmt::Result {
    out.push('[');
    out.write_fmt(what)
}

/// Adds one, and says so: an operand with an effect, evaluated once.
fn bump(cell: &Cell<u32>) -> u32 {
    print!("bump ");
    cell.set(cell.get() + 1);
    cell.get()
}

struct Held<'a> {
    what: fmt::Arguments<'a>,
}

fn main() {
    said(format_args!("{:?}", Noisy(1)));
    twice(format_args!("{:?}", Noisy(2)));
    never(format_args!("{:?}", Noisy(3)));
    passed_on(format_args!("{:?} {}", Noisy(4), 5));
    said(format_args!("{} {:?}", {
        print!("operand ");
        6
    }, Noisy(7)));

    let cell = Cell::new(1);
    let shows = Shows(&cell);
    let held = Held { what: format_args!("{shows}") };
    cell.set(2);
    println!("held: {}", held.what);
    cell.set(3);
    said(held.what);

    let items = [Noisy(10), Noisy(11)];
    for (k, item) in items.iter().enumerate() {
        let what = format_args!("<{item:?}>");
        print!("made {k} ");
        said(what);
    }

    let effect = Cell::new(0);
    let counted = format_args!("{} {:?}", {
        effect.set(effect.get() + 1);
        effect.get()
    }, Noisy(12));
    twice(counted);
    println!("effect ran {}", effect.get());

    let bumped = format_args!("{} then {}", bump(&effect), effect.get());
    effect.set(10);
    twice(bumped);
    println!("bumped {}", effect.get());

    let caught = catch_unwind(|| said(format_args!("{:?}", Panics)));
    println!(" caught {}", caught.is_err());

    let failed = catch_unwind(|| said(format_args!("{:?}", Bad)));
    let message = failed.unwrap_err();
    println!(" {:?}", message.downcast_ref::<String>());

    let mut out = String::new();
    let r = put(&mut out, format_args!("a{:?}b", Bad));
    println!("{r:?} {out:?}");

    println!("{:?} {:?}", format_args!("plain").as_str(), format_args!("{}", 1).as_str());
}
