// `Option`'s and `Result`'s combinators that move a value with a destructor
// into their closure: the closure owns it, and drops what it doesn't keep,
// as std's do.

struct Noisy(u8);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn some(n: u8) -> Option<Noisy> {
    Some(Noisy(n))
}

fn ok(n: u8) -> Result<Noisy, Noisy> {
    if n % 2 == 0 { Ok(Noisy(n)) } else { Err(Noisy(n)) }
}

fn half(n: u8) -> Result<u8, Noisy> {
    if n % 2 == 0 { Ok(n / 2) } else { Err(Noisy(n)) }
}

fn main() {
    println!("{}", some(1).is_some_and(|n| n.0 == 1));
    println!("{}", some(2).is_none_or(|n| n.0 > 5));
    println!("{}", ok(4).is_ok_and(|n| n.0 == 4));
    println!("{}", ok(5).is_err_and(|n| n.0 == 5));
    let kept = some(6).and_then(|n| if n.0 > 0 { Some(n) } else { None });
    println!("{}", kept.map_or(0, |n| n.0));
    println!("{}", some(7).map_or_else(|| 0, |n| n.0 + 1));
    println!("{}", ok(8).map_or_else(|e| e.0, |n| n.0 * 2));
    println!("{}", ok(9).unwrap_or_else(|e| Noisy(e.0 + 1)).0);
    println!("{:?}", ok(10).ok().map(|n| n.0));
    println!("{} {}", ok(11).is_ok_and(|n| n.0 > 0), ok(12).is_err_and(|n| n.0 > 0));
    println!("{:?} {:?}", ok(13).ok().map(|n| n.0), ok(14).err().map(|n| n.0));
    println!("{}", ok(15).map_or(Noisy(100), |n| n).0);
    println!("{}", ok(16).map_or(Noisy(101), |n| n).0);
    println!("{} {}", half(17).is_ok_and(|n| n > 0), half(18).is_err_and(|n| n.0 > 0));
    println!("end");
}
