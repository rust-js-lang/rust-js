// The greatest or least item by a comparison or a key, of items that may
// look like `None`: an `Option`'s `None` that's the extreme is an item,
// `Some(None)`, never no item, so the item is boxed (ADR 0051), each way
// the helper takes, `max_by`, `min_by`, `max_by_key`, `min_by_key`.
fn main() {
    let items = [None, Some(3), None];
    let max = items.iter().copied().max_by(|a, b| a.is_none().cmp(&b.is_none()));
    let min = items.iter().copied().min_by(|a, b| a.is_some().cmp(&b.is_some()));
    let by_key = items.iter().copied().max_by_key(|item| item.is_none());
    let least_key = items.iter().copied().min_by_key(|item| item.unwrap_or(-1));
    let none: [Option<u8>; 0] = [];
    let empty = none.iter().copied().max_by(|a, b| a.cmp(b));
    println!("{max:?} {min:?} {by_key:?} {least_key:?} {empty:?}");
}
