// A `&mut Duration` is a box of its nanoseconds, as a number's is: what's
// assigned through it is the caller's (ADRs 0074, 0188). Its `+=` and `-=`
// are its own `+` and `-`, which panic past zero.
use std::time::Duration;

fn bump(d: &mut Duration) {
    *d = *d + Duration::from_secs(1);
}

fn shorten(d: &mut Duration) {
    *d -= Duration::from_millis(250);
}

fn main() {
    let mut d = Duration::from_millis(500);
    bump(&mut d);
    println!("{:?}", d);
    shorten(&mut d);
    d += Duration::from_millis(5);
    println!("{:?}", d);
}
