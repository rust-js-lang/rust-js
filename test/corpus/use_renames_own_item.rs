// A module's `use`s that rename its own item, `pub(super) use self::Foo as
// Bar;`, import nothing: its file declares it, so two names of it are no
// two names of one import, as a `use` of another module's item would be
// (ADR 0202, amended; rustc's `privacy/pub-restricted-path-usage-55376.rs`).
use self::my_mod::Foo;

mod my_mod {
    pub(super) use self::Foo as Bar;
    pub(super) use self::count as tally;
    pub(in super::my_mod) use self::Foo as Baz;
    pub(in super::my_mod) use self::count as total;

    pub struct Foo;

    pub fn count() -> u32 {
        3
    }

    pub fn both() -> u32 {
        let _ = (Bar, Baz);
        tally() + total()
    }
}

fn main() {
    let _ = Foo;
    let _ = my_mod::Bar;
    println!("{} {}", my_mod::tally(), my_mod::both());
}
