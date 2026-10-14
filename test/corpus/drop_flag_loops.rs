// A variable assigned each time round a loop, and moved only some times:
// what's there when it's assigned again is dropped, and only then, as its
// flag says each time round, not as it was before the loop (ADR 0372).
struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn consume(n: Noisy) {
    println!("consume {}", n.0);
}

fn main() {
    let mut x;
    for i in 0..4 {
        x = Noisy(i);
        if i % 2 == 1 {
            consume(x);
        }
        println!("round {i}");
    }
    println!("done");
}
