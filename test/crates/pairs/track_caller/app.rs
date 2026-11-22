// The library's functions, called here: each given this call's location;
// and its trait's tracked method, through generic code, of its type and of
// one implemented here; and its trait's without a default, so (#2350).
use dep::Tracked;

#[track_caller]
fn passed_on() -> u32 {
    dep::line_here()
}

struct There;

impl Tracked for There {
    fn line(&self) -> u32 {
        std::panic::Location::caller().line() + 1000
    }
}

fn generic<T: Tracked>(t: &T) -> u32 {
    t.line()
}

struct Lax;

impl dep::Checked for Lax {
    fn checked(&self, n: u32) -> u32 {
        n
    }
}

fn check<C: dep::Checked>(c: &C) -> u32 {
    c.checked(2)
}

pub fn main() {
    println!("{} {}", dep::line_here(), dep::column_here());
    println!("{}", dep::quiet(1));
    println!("{}", passed_on());
    println!("{} {}", generic(&dep::Here), generic(&There));
    println!("{} {}", dep::Here.line(), There.line());
    println!("{:?}", dep::lines());
    println!("{} {}", check(&dep::Strict), check(&Lax));
    println!("{} {}", dep::check_one(&dep::Strict), dep::check_one(&Lax));
}
