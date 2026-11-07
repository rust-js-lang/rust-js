// A `Cycle` closed drops what it keeps, then what it steps, as its drop
// glue drops `orig`, then `iter`: the second though the first's drop
// panics, as drop glue goes on as one field's drop unwinds (#883).
use std::panic;

#[derive(Clone)]
struct Steps(u32, u32);

impl Iterator for Steps {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.1 < 2 {
            self.1 += 1;
            Some(self.1)
        } else {
            None
        }
    }
}

impl Drop for Steps {
    fn drop(&mut self) {
        println!("drop {} at {}", self.0, self.1);
        if self.0 == 1 && self.1 == 0 {
            panic!("the kept one's drop panics");
        }
    }
}

fn main() {
    let dropped = panic::catch_unwind(|| {
        let mut cycled = Steps(1, 0).cycle();
        println!("{:?}", cycled.next());
        drop(cycled);
    });
    println!("panicked: {}", dropped.is_err());
    let mut cycled = Steps(2, 0).cycle();
    println!("{:?}", cycled.next());
    drop(cycled);
    let taken: Vec<u32> = Steps(3, 0).cycle().take(5).collect();
    println!("{taken:?}");
}
