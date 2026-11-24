//! A library's fieldless enum iterator given its generic code, which holds
//! it in a place of its own, `{ value }` (#2255): inside std's `Take`, shown,
//! whose compiled `Debug` writes it into its sink through the consumer's
//! dictionary's `fmtInto`, given the place's value as `fmt` is, never the
//! place (#2545); and cloned or copied, a new place, where the export takes
//! none, so the consumer compiles its own instance (#2540).
use dep::{Light, cloned, copied, shown};

pub fn main() {
    println!("{}", shown(Light::Red.take(3)));
    println!("{}", shown(Light::Green.skip(1)));
    println!("{}", shown(Light::Green));
    println!("{:?}", cloned(Light::Red));
    println!("{:?}", copied(Light::Green));
}
