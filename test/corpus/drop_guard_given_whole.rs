// A tuple struct with a destructor given whole to a function, not only
// read by its field, stays the array it is: the function reads it as it is,
// and it's dropped after.
struct Seen(u32);

impl Drop for Seen {
    fn drop(&mut self) {
        println!("seen {}", self.0);
    }
}

#[inline(never)]
fn show(seen: &Seen) -> u32 {
    seen.0 * 2
}

fn shown(id: u32) -> u32 {
    let seen = Seen(id);
    show(&seen)
}

fn main() {
    println!("{}", shown(4));
}
