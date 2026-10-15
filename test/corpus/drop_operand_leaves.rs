//@ run-fail: attempt to divide by zero
// A value made for a call, before an operand that can leave, is the
// statement's temporary: the operand's `?`, call or division leaving drops
// it as it leaves. One that can't, a division by a literal other than 0,
// and of a signed number other than -1 too, leaves it the call's (ADR 0098).
struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn pair(a: Noisy, b: u32) -> u32 {
    println!("pair {} {}", a.0, b);
    b
}

fn half(n: u32) -> Option<u32> {
    if n % 2 == 0 { Some(n / 2) } else { None }
}

fn halves(n: u32) -> Option<u32> {
    Some(pair(Noisy("halved"), half(n)?))
}

fn checked(n: u32) -> u32 {
    println!("checked {n}");
    n
}

fn divided(n: u32, by: u32) -> u32 {
    let checked = pair(Noisy("checked"), checked(n));
    let halved = pair(Noisy("by two"), n / 2);
    pair(Noisy("by a variable"), n / by) + halved + checked
}

fn signed(a: Noisy, b: i32) -> i32 {
    println!("signed {} {}", a.0, b);
    b
}

fn quartered(n: i32) -> i32 {
    signed(Noisy("by four"), n / 4) + signed(Noisy("by minus two"), n % -2)
}

fn main() {
    println!("{}", quartered(-9));
    println!("{:?}", halves(4));
    println!("{:?}", halves(3));
    println!("{}", divided(8, 4));
    println!("{}", divided(8, 0));
}
