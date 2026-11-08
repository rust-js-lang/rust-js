//@ library-differs
// A temporary made a `&mut dyn` of a generic impl, `&mut Count { n: 3 }`,
// whose pair writes its value in place (#968): one value, named once, read
// and written each time it's stepped, not a new one at each read, which
// never ran down (#985).
#[derive(Debug)]
struct Count {
    n: u32,
}

trait GetMut<T> {
    fn get_mut(&mut self) -> &mut T;
}

impl<T> GetMut<T> for T {
    fn get_mut(&mut self) -> &mut T {
        self
    }
}

impl Count {
    fn next_item(&mut self) -> Option<u32> {
        if self.n == 0 {
            return None;
        }
        self.n -= 1;
        Some(self.n)
    }
}

fn drain(s: &mut dyn GetMut<Count>) -> Vec<u32> {
    let s = s.get_mut();
    let mut out = Vec::new();
    while let Some(x) = s.next_item() {
        out.push(x);
    }
    out
}

fn replaced(s: &mut dyn GetMut<Count>) -> u32 {
    *s.get_mut() = Count { n: 7 };
    s.get_mut().n
}

fn main() {
    println!("{:?}", drain(&mut Count { n: 3 }));
    println!("{}", replaced(&mut Count { n: 1 }));
    let mut kept = Count { n: 2 };
    println!("{:?} {kept:?}", drain(&mut kept));
}
