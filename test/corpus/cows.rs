// A `Cow` is its enum, `{ TAG, _0 }` (ADR 0033): made owned by
// `into_owned()` or `to_mut()`, a clone of what it borrowed.

use std::borrow::Cow;
use std::path::Path;

fn shout(text: &str) -> Cow<'_, str> {
    if text.ends_with('!') {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(format!("{text}!"))
    }
}

fn main() {
    let hi = shout("hi");
    let hey = shout("hey!");
    println!("{hi} {hey} {:?} {}", hey, matches!(hey, Cow::Borrowed(_)));
    let owned: String = hey.clone().into_owned();
    println!("{owned} {}", hi.into_owned());

    let mut text = shout("wow!");
    text.to_mut().push_str("!!");
    println!("{text} {}", matches!(text, Cow::Owned(_)));

    let items = [1, 2, 3];
    let mut list: Cow<'_, [i32]> = Cow::Borrowed(&items);
    println!("{:?}", list);
    list.to_mut().push(4);
    list.to_mut()[0] = 0;
    println!("{:?} {:?} {}", list, items, list.len());
    let vec: Vec<i32> = list.into_owned();
    println!("{vec:?}");
    let borrowed: Cow<'_, [i32]> = Cow::Borrowed(&items);
    let mut copy = borrowed.into_owned();
    copy.push(9);
    println!("{copy:?} {items:?} {:?}", Cow::<[i32]>::Owned(copy.clone()));
    let path: Cow<'_, Path> = Cow::Borrowed(Path::new("a/b"));
    let buf = path.clone().into_owned();
    println!("{} {:?} {}", path.display(), path, buf.display());
}
