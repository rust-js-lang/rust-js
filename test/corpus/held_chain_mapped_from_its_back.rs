// A map held in a variable of a chain of the crate's iterators with a
// destructor, stepped from its back: the chain the map holds is made
// knowing its back where it's made, though its items drop nothing, and so
// is another of its type, held in a variable before it's mapped and
// reversed (#739, #718).

struct Own(u32, &'static str);

impl Drop for Own {
    fn drop(&mut self) {
        println!("drop {}", self.1);
    }
}

impl Iterator for Own {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(self.0)
    }
}

impl DoubleEndedIterator for Own {
    fn next_back(&mut self) -> Option<u32> {
        self.next()
    }
}

fn main() {
    let mut it = Own(3, "x").chain(Own(1, "y")).map(|x| x + 5);
    println!("{:?} {:?}", it.next_back(), it.next());
    drop(it);
    let c = Own(2, "a").chain(Own(2, "b"));
    let v: Vec<u32> = c.map(|x| x + 10).rev().collect();
    println!("{v:?}");
}
