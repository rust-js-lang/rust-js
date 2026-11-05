// A `skip(n)` or a `take(n)` made in place and stepped from its back by
// what takes it through a `&mut`, `next_back`, `nth_back` or `rfind`,
// which MIR holds in a variable to lend it: made knowing its back where
// it's made, `$taking` or `$skipping`, of a slice's items, a range, a map,
// a copy and a `Vec`'s items, of numbers and of items with a destructor,
// which what it passes over drops (#718).
struct L(u32);

impl Drop for L {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn ls(n: u32) -> Vec<L> {
    (1..=n).map(L).collect()
}

fn main() {
    let v = vec![1, 2, 3, 4, 5];
    println!("{:?}", v.iter().take(2).next_back());
    println!("{:?}", v.iter().take(3).nth_back(1));
    println!("{:?}", v.iter().take(2).rfind(|x| **x > 1));
    println!("{:?}", v.iter().skip(2).next_back());
    println!("{:?}", v.iter().skip(2).rfind(|x| **x < 5));
    println!("{:?}", v.iter().skip(4).nth_back(1));
    println!("{:?}", (0..9).take(3).rfind(|x| x % 2 == 0));
    println!("{:?}", (1..10).skip(3).take(4).rfind(|x| x % 3 == 0));
    println!("{:?}", v.iter().map(|x| x * 2).take(3).rfind(|x| x % 4 == 0));
    println!("{:?}", v.iter().copied().skip(1).rfind(|x| x % 2 == 0));
    let w = vec![1u32, 2, 3, 4, 5];
    println!("{:?}", w.into_iter().take(3).rfind(|x| *x < 3));
    println!("{:?}", ls(4).into_iter().take(2).rfind(|l| l.0 > 1).map(|l| l.0));
    println!("{:?}", ls(5).into_iter().skip(1).rfind(|l| l.0 % 2 == 0).map(|l| l.0));
    println!("{:?}", ls(6).into_iter().take(4).nth_back(1).map(|l| l.0));
    println!("{:?}", ls(6).into_iter().skip(2).next_back().map(|l| l.0));
    println!("{:?}", ls(7).into_iter().skip(1).take(4).nth_back(2).map(|l| l.0));
    // Held in a variable, std's struct, beside them.
    let mut it = v.iter().skip(1);
    println!("{:?} {:?}", it.rfind(|x| **x % 2 == 0), it.next());
}
