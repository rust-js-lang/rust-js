//! A trait's generic default whose function inside it indexes with `..`,
//! as base64's `Engine::encode` does (#474), and makes a unit struct of the
//! library's own.
pub trait Engine {
    fn letter(&self) -> u8;

    fn encode<T: AsRef<[u8]>>(&self, input: T) -> String {
        fn inner<E: Engine + ?Sized>(engine: &E, input: &[u8]) -> String {
            let mut buf = vec![0; input.len()];
            fill(&mut buf[..], engine.letter(), Marker);
            let tail = &input[..];
            format!("{}{}", String::from_utf8(buf).expect("ASCII"), tail.len())
        }
        inner(self, input.as_ref())
    }
}

#[derive(Debug)]
pub struct Marker;

fn fill(buf: &mut [u8], letter: u8, marker: Marker) {
    for (i, slot) in buf.iter_mut().enumerate() {
        *slot = letter + i as u8;
    }
    println!("{marker:?}");
}

pub struct Letters(pub u8);

impl Engine for Letters {
    fn letter(&self) -> u8 {
        self.0
    }
}
