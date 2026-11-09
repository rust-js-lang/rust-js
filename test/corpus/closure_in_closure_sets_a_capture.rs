// A closure in a closure that sets a variable both capture, `d = D(..)`:
// the outer one holds it as `d$1`, `const d$1 = d`, and the inner one's new
// value, named after its type, is `d`, its own. The outer one's `d$1` is
// read as `d` only where nothing after declares a `d` of its own, which the
// read would be of instead: `$assign(d, d)` set the new value to itself,
// the old one never dropped there and `d` never set (#1078, the
// metamorphic net's `for-each` rewrite of `drop_reassigned_in_inner_loop`).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn nested(n: u32) -> u32 {
    let mut d = D(24);
    let mut seen = 0;
    (0..n).for_each(|k| {
        (0..2u32).for_each(|j| {
            d = D(k * 10 + j);
            seen += d.0;
        });
    });
    seen + d.0
}

fn main() {
    println!("{}", nested(2));
    println!("{}", nested(0));
}
