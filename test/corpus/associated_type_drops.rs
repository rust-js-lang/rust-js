// A value of an associated type, dropped in generic code, where the impl's
// type has a destructor and where it hasn't (ADR 0178).

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

trait Zone {
    type Offset;
    fn offset(&self, name: &'static str) -> Self::Offset;
    fn show(offset: &Self::Offset) -> String;
}

struct Loud;

impl Zone for Loud {
    type Offset = Noisy;
    fn offset(&self, name: &'static str) -> Noisy {
        Noisy(name)
    }
    fn show(offset: &Noisy) -> String {
        format!("loud {}", offset.0)
    }
}

struct Quiet;

impl Zone for Quiet {
    type Offset = u32;
    fn offset(&self, name: &'static str) -> u32 {
        name.len() as u32
    }
    fn show(offset: &u32) -> String {
        format!("quiet {offset}")
    }
}

// As chrono's `DateTime<Tz>` holds a `Tz::Offset`.
struct Stamp<Z: Zone> {
    at: i64,
    offset: Z::Offset,
}

fn stamp<Z: Zone>(zone: &Z, at: i64, name: &'static str) -> Stamp<Z> {
    Stamp { at, offset: zone.offset(name) }
}

fn describe<Z: Zone>(zone: &Z) {
    let first = zone.offset("first");
    println!("made {}", Z::show(&first));
    let second = stamp(zone, 7, "second");
    println!("stamp {} {}", second.at, Z::show(&second.offset));
    let maybe: Option<Z::Offset> = Some(zone.offset("maybe"));
    println!("some {}", maybe.is_some());
    {
        let inner = zone.offset("inner");
        println!("inner {}", Z::show(&inner));
    }
    let mut replaced = zone.offset("old");
    println!("was {}", Z::show(&replaced));
    replaced = zone.offset("new");
    println!("replaced {}", Z::show(&replaced));
    println!("end");
}

// Through a `dyn`, whose type names the item's.
trait Source {
    type Item;
    fn make(&self) -> Self::Item;
}

impl Source for Loud {
    type Item = Noisy;
    fn make(&self) -> Noisy {
        Noisy("made")
    }
}

fn take<S: Source + ?Sized>(source: &S) {
    let _item = source.make();
    println!("taken");
}

fn main() {
    describe(&Loud);
    describe(&Quiet);
    let kept = stamp(&Loud, 1, "kept");
    println!("kept {}", kept.at);
    let source: &dyn Source<Item = Noisy> = &Loud;
    take(source);
    take(&Loud);
}
