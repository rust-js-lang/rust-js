//@ compile-fail: user implementations of `std::borrow::ToOwned`
// A `Cow` of a type whose `ToOwned` is its own may own something else than a
// clone of what it borrows: refused (ADR 0319).

use std::borrow::{Borrow, Cow};

struct Name;
struct OwnedName(String);

impl Borrow<Name> for OwnedName {
    fn borrow(&self) -> &Name {
        &Name
    }
}

impl ToOwned for Name {
    type Owned = OwnedName;
    fn to_owned(&self) -> OwnedName {
        OwnedName("made".to_string())
    }
}

fn main() {
    let name = Name;
    let cow: Cow<'_, Name> = Cow::Borrowed(&name);
    println!("{}", cow.into_owned().0);
}
