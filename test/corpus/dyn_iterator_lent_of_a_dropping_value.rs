// A `&mut dyn Iterator` of a value with a destructor owns nothing to drop:
// it's lent, and its owner drops the value once, where Rust does.
struct Countdown(u32);

impl Drop for Countdown {
    fn drop(&mut self) {
        println!("dropped at {}", self.0);
    }
}

impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            None
        } else {
            self.0 -= 1;
            Some(self.0)
        }
    }
}

fn first_two(it: &mut dyn Iterator<Item = u32>) -> Vec<u32> {
    it.take(2).collect()
}

fn main() {
    let mut c = Countdown(5);
    println!("{:?}", first_two(&mut c));
    let lent: &mut dyn Iterator<Item = u32> = &mut c;
    println!("{:?}", lent.next());
    println!("after");
}
