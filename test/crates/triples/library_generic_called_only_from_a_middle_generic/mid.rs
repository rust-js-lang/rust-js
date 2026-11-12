//! A library that calls the first one's generic functions from its own
//! generic code alone, each given its `Up`, which drops: it compiles its own
//! copy of each, which its generic code reaches (#1596).
pub struct Up {
    pub n: u32,
}

impl Iterator for Up {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.n < 3 {
            self.n += 1;
            Some(self.n)
        } else {
            None
        }
    }
}

impl Drop for Up {
    fn drop(&mut self) {
        println!("drop Up {}", self.n);
    }
}

pub fn up<C: Extend<u32>>(c: C) -> C {
    dep::given(c, Up { n: 0 })
}

pub fn first_of<T>(_: T) -> Option<u32> {
    dep::first(Up { n: 1 })
}
