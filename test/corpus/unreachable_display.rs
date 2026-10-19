//@ edition: 2018
//@ run-fail: internal error: entered unreachable code: 7 is too big
// An edition 2018 `unreachable!(x)` is a panic of what `x` shows, after
// std's words, as edition 2021's `unreachable!("{}", x)` is; base64's
// `unreachable!("Vec is sized conservatively")`.
struct Reason(u32);

impl std::fmt::Display for Reason {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} is too big", self.0)
    }
}

fn check(n: u32) -> u32 {
    match n {
        0..=5 => n * 2,
        6 => unreachable!("six"),
        _ => unreachable!(Reason(n)),
    }
}

fn main() {
    println!("{}", check(3));
    check(7);
}
