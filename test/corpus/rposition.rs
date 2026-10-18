// `rposition(f)` of a slice's items: where the last that `f` finds is,
// each tested from the back as Rust tests them; of an iterator kept and
// stepped from its front, of what's left, which it leaves stepped from its
// back to what it found (ADR 0164).
fn main() {
    let v = vec![5, 3, 8, 1, 8];
    let last_big = v.iter().rposition(|&x| { println!("test {x}"); x > 4 });
    let none = v.iter().rposition(|&x| x > 100);
    let first_big = v.iter().position(|&x| x > 4);
    let s: &[u8] = b"a,b,c";
    println!("{last_big:?} {none:?} {first_big:?} {:?}", s.iter().rposition(|&b| b == b','));
    kept();
}
fn kept() {
    let v = [1, 9, 2, 9, 3];
    let mut it = v.iter();
    it.next();
    let at = it.rposition(|&x| x == 9);
    println!("{at:?} {:?}", it.collect::<Vec<_>>());
}
