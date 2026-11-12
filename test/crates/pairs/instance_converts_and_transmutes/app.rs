use dep::{describe, first, read_lanes, write_lanes};

struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub fn main() {
    let input: Vec<u8> = (1..=16).collect();
    let (lanes, kept) = read_lanes(&input, Noisy(1)).unwrap();
    println!("{lanes:?} {}", kept.0);
    let mut out = vec![0u8; 16];
    let kept = write_lanes(lanes, &mut out, kept);
    println!("{out:?} {}", kept.0);
    println!("{:?} {:?}", first(&input), first(&input[..2]));
    for len in [3, 17] {
        let bytes: Vec<u8> = (0..len).collect();
        match read_lanes(&bytes, Noisy(len as u32)) {
            Ok(_) => println!("read {len}"),
            Err(error) => {
                let (said, kept) = describe(&error, Noisy(error.code.get()));
                println!("{len}: {} {said} {}", error.code, kept.0);
            }
        }
    }
}
