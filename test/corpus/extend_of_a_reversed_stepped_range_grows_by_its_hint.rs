// A `Vec` extended by a range stepped from its end, `(0..n).rev().step_by(2)`,
// grows by the room its hint asks, as std's `extend` reserves by
// `size_hint()` first (#1046): its capacity std's, 5 of `0..10`, not one a
// push at a time would leave. And of a range of `char`s and an inclusive
// one, and a hint asked after a step.
fn main() {
    let n = 10u32;
    let mut v = Vec::new();
    v.extend((0..n).rev().step_by(2));
    println!("{:?} {}", v, v.capacity());
    let mut w = vec![0u32];
    w.extend((0..=n).rev().step_by(3));
    println!("{:?} {}", w, w.capacity());
    let mut c = Vec::new();
    c.extend(('a'..'k').rev().step_by(4));
    println!("{:?} {}", c, c.capacity());
}
