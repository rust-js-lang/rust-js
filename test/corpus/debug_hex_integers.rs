// `{:x?}` and `{:X?}` of an integer: `Debug` with its digits in hex, as
// aho-corasick's `Transition` shows its byte.

struct Transition {
    byte: u8,
    next: usize,
}

impl std::fmt::Debug for Transition {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Transition(byte: {:X?}, next: {:?})", self.byte, self.next)
    }
}

fn main() {
    println!("{:x?} {:X?} {:#x?} {:4x?}| {:x?}", 255u8, 3054u32, 16i32, 10u16, -1i8);
    println!("{:?}", Transition { byte: 0xab, next: 7 });
}
