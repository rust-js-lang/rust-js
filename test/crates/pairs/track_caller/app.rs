// The library's functions, called here: each given this call's location.
#[track_caller]
fn passed_on() -> u32 {
    dep::line_here()
}

pub fn main() {
    println!("{} {}", dep::line_here(), dep::column_here());
    println!("{}", dep::quiet(1));
    println!("{}", passed_on());
}
