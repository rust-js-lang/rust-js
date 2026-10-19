// A type of a const parameter, `Buffer<SIZE>`, asked whether its drop runs
// something where that parameter is: a generic function's caller giving it
// on, and a trait's default copied into its impl (powerfmt's `WriteBuffer`).

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[derive(Clone, Copy)]
struct Buffer<const SIZE: usize> {
    bytes: [u8; SIZE],
    len: usize,
}

impl<const SIZE: usize> Buffer<SIZE> {
    fn new() -> Self {
        Buffer {
            bytes: [0; SIZE],
            len: 0,
        }
    }
}

fn keep<T>(value: T) -> T {
    value
}

fn made<const SIZE: usize>() -> usize {
    let buffer = keep(Buffer::<SIZE>::new());
    buffer.bytes.len() + buffer.len
}

trait Described {
    fn size(&self) -> usize;

    fn described(self) -> String
    where
        Self: Sized,
    {
        format!("{} bytes", self.size())
    }
}

impl<const SIZE: usize> Described for Buffer<SIZE> {
    fn size(&self) -> usize {
        self.bytes.len()
    }
}

fn main() {
    let loud = Loud(1);
    println!("{} {}", made::<4>(), Buffer::<8>::new().described());
    println!("{}", loud.0);
}
