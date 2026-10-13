// A function declared in a body (ADR 0308) is prepared as the one it's in:
// `s += t` of a string built up, and no `else` after a branch that leaves.
fn outer(n: u32) -> String {
    fn inner(x: u32) -> String {
        let mut s = String::new();
        for i in 0..x {
            s += &i.to_string();
        }
        if s.len() > 2 {
            return s;
        } else {
            let mut t = s.clone();
            t += "!";
            return t;
        }
    }
    let mut t = String::new();
    for i in 0..n {
        t += &i.to_string();
    }
    inner(n) + "|" + &t
}

fn main() {
    println!("{} {}", outer(2), outer(4));
}
