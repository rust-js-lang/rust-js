// A chain the crate clones is cloned apart (ADR 0525), each part cloned:
// one whose other side is given as what makes an iterator, `chain(&b)` of
// a `Vec`, takes its iterator where the chain is made, so its clone clones
// how far it's stepped, and one of that type the crate never clones,
// `chain(&b).sum()`, is made so too, where it was refused (#1002).
fn main() {
    let a = [1, 2, 3];
    let b = vec![10, 20];
    let mut chain = a.iter().chain(&b);
    chain.next();
    chain.next();
    chain.next();
    chain.next();
    let copy = chain.clone();
    println!("{:?} {:?}", chain.collect::<Vec<_>>(), copy.collect::<Vec<_>>());
    let s: i32 = a.iter().chain(&b).sum();
    let mut stepped = a.iter().chain(&b);
    stepped.nth(3);
    let again = stepped.clone();
    println!("{s} {:?} {:?}", stepped.next(), again.count());
}
