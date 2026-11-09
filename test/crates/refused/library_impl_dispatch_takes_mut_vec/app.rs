//! Refused as the library's own dictionary of `H<T>` given an object is:
//! the impl's entry takes its objects as they are, and generic code would
//! give it boxes.
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
    let mut h = dep::H { a: C { n: 2 }, b: C { n: 5 } };
    dep::go(&mut h);
    println!("{} {}", h.a.n, h.b.n);
}
