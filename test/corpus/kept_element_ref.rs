// A `&mut` to an element is to the element its index named when it was
// made, though the index changes after (ADR 0364).
fn main() {
    let mut a = [1u32, 2, 3];
    let mut i = 0;
    let r = &mut a[i];
    i = 2;
    *r += 10;
    println!("{:?} {}", a, i);
}
