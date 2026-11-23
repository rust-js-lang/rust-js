// `Seek::rewind` its type's impl writes itself is that impl's, called on a
// type parameter too, as Rust's dispatch picks (#2414).
use std::io::{Seek, SeekFrom};

struct Tape(u64);

impl Seek for Tape {
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        if let SeekFrom::Start(n) = to {
            self.0 = n;
        }
        Ok(self.0)
    }
    fn rewind(&mut self) -> std::io::Result<()> {
        println!("own rewind");
        self.0 = 7;
        Ok(())
    }
}

fn reset<S: Seek>(s: &mut S) {
    s.rewind().unwrap();
}

fn main() {
    let mut tape = Tape(3);
    reset(&mut tape);
    println!("{}", tape.0);
}
