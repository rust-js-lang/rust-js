// An `Option`'s `iter().count()` is how many it holds, `o == null ? 0 :
// 1`, reading the `Option` once, so a tuple of it keeps its elements in
// place, `[f() != null, g() != null, h() == null ? 0 : 1]`, not held in
// `const`s before it (#588). Of an `Option` of an `Option`, its `Some(None)`
// is one too.
fn some_of(items: &[u8]) -> Option<u8> {
    items.first().copied()
}

fn nested(items: &[u8]) -> Option<Option<u8>> {
    items.first().map(|&n| if n > 1 { Some(n) } else { None })
}

pub fn counted(a: &[u8], b: &[u8]) -> (bool, bool, usize) {
    (some_of(a).is_some(), some_of(b).is_none(), some_of(a).iter().count())
}

pub fn counted_nested(a: &[u8]) -> usize {
    nested(a).iter().count()
}

fn main() {
    println!("{:?} {:?}", counted(&[1], &[]), counted(&[], &[2]));
    println!("{} {} {}", counted_nested(&[0]), counted_nested(&[5]), counted_nested(&[]));
}
