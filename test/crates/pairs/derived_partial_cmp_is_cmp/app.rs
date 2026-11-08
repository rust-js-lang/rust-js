use dep::{Plain, Split, Twisted, Whole};

fn less<T: PartialOrd>(a: &T, b: &T) -> bool {
    a < b
}

pub fn main() {
    let (a, b) = (Whole(Twisted(1)), Whole(Twisted(2)));
    println!("whole");
    println!("{:?} {}", a.partial_cmp(&b), a < b);
    println!("{}", less(&a, &b));
    let (c, d) = (Split(Twisted(1)), Split(Twisted(2)));
    println!("split");
    println!("{:?} {}", c.partial_cmp(&d), c < d);
    println!("{}", less(&c, &d));
    let (e, f) = (Plain(Twisted(1)), Plain(Twisted(2)));
    println!("plain");
    println!("{:?} {}", e.partial_cmp(&f), e < f);
    println!("{}", less(&e, &f));
}
