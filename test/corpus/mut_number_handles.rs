// A `&mut` to a number kept in a struct, chosen by a branch or returned
// is a handle on its place (ADR 0099): it reads and writes the place.
struct Holder<'a> {
    count: &'a mut u32,
}

fn pick<'a>(take_first: bool, a: &'a mut u32, b: &'a mut u32) -> &'a mut u32 {
    if take_first { a } else { b }
}

fn main() {
    let (mut a, mut b) = (1u32, 2u32);
    *pick(true, &mut a, &mut b) += 10;
    *pick(false, &mut a, &mut b) += 20;
    let h = Holder { count: &mut a };
    *h.count *= 2;
    let r = if a > 5 { &mut a } else { &mut b };
    *r += 1;
    println!("{a} {b}");
}
