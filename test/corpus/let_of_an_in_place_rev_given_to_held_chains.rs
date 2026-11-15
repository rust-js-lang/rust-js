// A `let` of a `rev()` read once, by a chain the transparent-chain
// predicate doesn't admit, a zip, a map, a chain: the ground makes the
// `rev()` in place, a JS iterator `back_stepped` reads as it is, and the
// held flip, which may take a `rev()` out after `back_stepped`, gives no
// compiled adapter it, so each chain on it stays the recognizers' form and
// the flip never changes what decided it (ADR 1136, amended).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn plus(g: &D, x: u32) -> u32 {
    g.0 + x
}

fn main() {
    let v = [1u32, 2, 3];
    let w = [10u32, 20, 30, 40];
    let g = D(5);
    let back = v.iter().rev();
    let pairs: Vec<(&u32, u32)> = back.zip(w.iter().map(move |x| plus(&g, *x))).collect();
    println!("zip {pairs:?}");

    let g = D(6);
    let back = (0..4u32).rev();
    let sum = back.map(move |x| plus(&g, x)).fold(0, |s, x| s * 100 + x);
    println!("map {sum}");

    let g = D(7);
    let back = v.iter().rev();
    let firsts: Vec<(usize, u32)> = back.map(move |x| plus(&g, *x)).enumerate().collect();
    println!("enumerate {firsts:?}");

    let back = v.iter().rev();
    let chained: Vec<&u32> = back.chain(w.iter().rev()).collect();
    println!("chain {chained:?}");
}
