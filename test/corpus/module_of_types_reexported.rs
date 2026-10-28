// A module of types alone, its `.d.ts` alone (ADR 1005), that another
// module's JS re-exports, `pub use foo::bar::{self}`, `export * as bar from
// "./foo/bar.js"`, keeps its JS, though it has nothing in it: a JS module
// importing one that isn't there throws as it loads (rustc's
// `imports/use-mod.rs`).

pub use foo::bar::{self, First};
use self::bar::Second;

mod foo {
    pub use self::bar::baz;

    pub mod bar {
        pub mod baz {
            pub struct Fourth;
        }
        pub struct First;
        pub struct Second;
    }

    pub struct Third;
}

mod baz {
    pub use super::foo::Third;
}

fn main() {
    let _ = (First, Second, baz::Third, foo::baz::Fourth);
    println!("made");
}
