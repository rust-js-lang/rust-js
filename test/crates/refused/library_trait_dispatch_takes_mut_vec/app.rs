//! Refused as the library's own call of `go` is: an impl's entry takes
//! its objects as they are, and generic code would give it boxes.
pub struct C {
    pub n: i32,
}

impl dep::Pick for C {
    fn pick(&mut self, o: Vec<&mut Self>) {
        for o in o {
            o.n += self.n;
        }
    }
}

pub fn main() {
    let (mut a, mut b) = (C { n: 2 }, C { n: 5 });
    dep::go(&mut a, &mut b);
    println!("{} {}", a.n, b.n);
}
