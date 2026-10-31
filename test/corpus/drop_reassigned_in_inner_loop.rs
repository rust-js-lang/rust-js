// A value given again in a loop inside another, its old one dropped there
// each time, `d = D(id(29))` (generated program 12454574): no `try { .. }
// finally { drop }` around the outer loop, which took that drop for one
// before a way out, the inner loop's `continue bb10` for the outer's, and
// dropped the value once, at the end, from outside its `let`'s scope. A
// range whose end is a call, which may panic and drop what's held, is still
// a `for`: its end held first in a `try` of its own, `let end; try { end =
// id(n); } catch ..`, then `for (let k = 0; k < end; k++)`, beside a
// variable of the program's own named `end`.
fn id<T>(x: T) -> T {
    x
}

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn nested(n: u32) -> u32 {
    let mut d = D(id(24u32));
    let mut seen = 0;
    for k in 0..id(n) {
        for j in 0..id(3u32) {
            d = D(id(k * 10 + j));
            seen += d.0;
        }
    }
    seen
}

fn fixed() -> u32 {
    let mut d = D(24);
    for k in 0..3u32 {
        for j in 0..3u32 {
            d = D(id(k * 10 + j));
        }
    }
    d.0
}

fn generated() {
    let mut d23 = D(id(24u32));
    for _k27 in 0..id(3u32) {
        for _k28 in 0..id(3u32) {
            d23 = D(id(29u32));
        }
    }
    let _ = &d23;
}

fn beside_an_end(n: u32) -> u32 {
    let mut d = D(id(5u32));
    let end = n * 2;
    let mut sum = 0;
    for k in 1..=id(n) {
        d = D(id(k));
        sum += k + end + d.0;
    }
    sum + end
}

fn main() {
    println!("{}", fixed());
    generated();
    println!("{}", nested(3));
    println!("{}", nested(0));
    println!("{}", beside_an_end(4));
}
