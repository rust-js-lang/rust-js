//! Each dropped where Rust drops it.
use dep::{Fragile, Gen, Loud, Slots, drop_at, holding, made_second, second};
use std::panic::catch_unwind;

pub fn main() {
    println!("kept {}", second(Gen(0)).map_or(0, |l| l.0));
    println!("made {}", made_second(Gen(0)).map_or(0, |l| l.0));
    {
        let count = holding(Loud(9));
        println!("holds {}", count());
    }
    let mut slots = Slots {
        items: vec![Loud(5), Loud(6)],
    };
    slots.truncate(0);
    println!("truncated {}", slots.items.len());
    let mut items = vec![Loud(7)];
    unsafe {
        items.set_len(0);
        drop_at(items.as_mut_ptr());
    }
    println!("dropped at {}", items.len());
    // What `zip` leaves dropped as the collect ends, the third's drop
    // panicking: the rest, then what was collected, dropped as it unwinds.
    let caught = catch_unwind(|| {
        let all: Vec<Fragile> = (1..6).map(Fragile).collect();
        all.into_iter().zip(0..2).collect::<Vec<_>>().len()
    });
    println!("caught {}", caught.is_err());
}
