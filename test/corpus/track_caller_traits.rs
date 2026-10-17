// A trait method whose declaration tracks its caller is given where it's
// called through a dictionary too, generic code's or a `dyn`'s: each entry
// of it takes `location`, after its arguments, as each impl does. An impl
// that tracks its caller where its declaration doesn't is given its own
// definition's, as rustc's shim of it gives it (ADR 0427).
use std::panic::Location;

trait Tracked {
    #[track_caller]
    fn from_default(&self) -> &'static Location<'static> {
        Location::caller()
    }

    #[track_caller]
    fn inherited(&self) -> &'static Location<'static>;

    fn only_impl(&self) -> &'static Location<'static>;

    #[track_caller]
    fn boxed(self: Box<Self>) -> &'static Location<'static>;

    #[track_caller]
    fn unread(&self) -> u32;

    #[track_caller]
    fn tagged<T: std::fmt::Display>(&self, tag: T) -> String
    where
        Self: Sized;
}

struct Unit;

impl Tracked for Unit {
    // Its declaration's `#[track_caller]` is its own too.
    fn inherited(&self) -> &'static Location<'static> {
        Location::caller()
    }

    #[track_caller]
    fn only_impl(&self) -> &'static Location<'static> {
        Location::caller()
    }

    fn boxed(self: Box<Self>) -> &'static Location<'static> {
        Location::caller()
    }

    fn unread(&self) -> u32 {
        7
    }

    fn tagged<T: std::fmt::Display>(&self, tag: T) -> String
    where
        Self: Sized,
    {
        format!("{tag} at {}", Location::caller().line())
    }
}

struct Flag(bool);

impl Tracked for Flag {
    #[track_caller]
    fn from_default(&self) -> &'static Location<'static> {
        let here = Location::caller();
        println!("flag {} called at line {}", self.0, here.line());
        here
    }

    fn inherited(&self) -> &'static Location<'static> {
        Location::caller()
    }

    fn only_impl(&self) -> &'static Location<'static> {
        Location::caller()
    }

    fn boxed(self: Box<Self>) -> &'static Location<'static> {
        Location::caller()
    }

    fn unread(&self) -> u32 {
        8
    }

    fn tagged<T: std::fmt::Display>(&self, tag: T) -> String
    where
        Self: Sized,
    {
        format!("{tag} of a flag")
    }
}

fn generic<T: Tracked>(t: &T) -> [&'static Location<'static>; 3] {
    println!("{}", t.tagged(5));
    [t.from_default(), t.inherited(), t.only_impl()]
}

#[track_caller]
fn passed_on<T: Tracked>(t: &T) -> &'static Location<'static> {
    t.inherited()
}

#[track_caller]
fn pointer_inside() -> &'static Location<'static> {
    let f: fn() -> &'static Location<'static> = here;
    f()
}

#[track_caller]
fn here() -> &'static Location<'static> {
    Location::caller()
}

fn main() {
    let unit = Unit;
    println!("{}", unit.from_default());
    println!("{}", unit.inherited());
    println!("{}", unit.only_impl());
    for at in generic(&unit) {
        println!("{at}");
    }
    for at in generic(&Flag(true)) {
        println!("{at}");
    }
    let objects: [&dyn Tracked; 2] = [&Unit, &Flag(false)];
    for object in objects {
        println!("{} {} {}", object.from_default(), object.inherited(), object.only_impl());
        println!("{}", object.unread());
    }
    println!("{}", passed_on(&unit));
    let boxed: Box<dyn Tracked> = Box::new(Unit);
    println!("{}", boxed.boxed());
    println!("{}", Box::new(Flag(true)).boxed());
    println!("{}", pointer_inside());
}
