//@ compile-fail: `{:?}` of a `TypeId`, its hash
// A `TypeId`'s `{:?}` shows its hash, which rust-js doesn't keep: a
// `TypeId` is its type's name (ADR 0331).

use std::any::TypeId;

fn main() {
    println!("{:?}", TypeId::of::<u8>());
}
