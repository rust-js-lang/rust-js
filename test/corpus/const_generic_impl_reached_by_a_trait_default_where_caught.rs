//@ library-differs
// A trait's default writer that reaches, through an impl for a
// const-generic type, `Buf<SIZE>`'s, a generic struct holding the impl's
// associated type, `Held<'_, Buf<SIZE>>`'s `T::Metadata`, in a program that
// may catch a panic. The impl's method's signature is asked of where it's
// declared, whose env has `SIZE`, not its caller's, and what a writer may
// drop is asked where its type's parameters are the walked body's own:
// rustc crashed on both, `cannot find SIZE in param-env` (#1618, #1608,
// powerfmt's shape).
use std::fmt;

trait Measured {
    type Metadata;

    fn metadata(&self, kept: bool) -> Option<Held<'_, Self>>;

    fn written(&self, f: &mut fmt::Formatter<'_>, held: Option<Held<'_, Self>>) -> fmt::Result;

    fn shown(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let held = self.metadata(true);
        self.written(f, held)
    }
}

struct Held<'a, T: Measured + ?Sized> {
    metadata: T::Metadata,
    of: &'a T,
}

impl<'a, T: Measured + ?Sized> Held<'a, T> {
    fn new(metadata: T::Metadata, of: &'a T, kept: bool) -> Option<Self> {
        match kept {
            true => Some(Held { metadata, of }),
            false => None,
        }
    }
}

struct Buf<const SIZE: usize> {
    bytes: [u8; SIZE],
}

impl<const SIZE: usize> Measured for Buf<SIZE> {
    type Metadata = Vec<usize>;

    fn metadata(&self, kept: bool) -> Option<Held<'_, Self>> {
        Held::new(vec![SIZE, self.bytes.len()], self, kept)
    }

    fn written(&self, f: &mut fmt::Formatter<'_>, held: Option<Held<'_, Self>>) -> fmt::Result {
        match held {
            Some(held) => write!(f, "{} of {}", held.metadata[1], held.metadata[0]),
            None => write!(f, "none"),
        }
    }
}

impl<const SIZE: usize> fmt::Display for Buf<SIZE> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.shown(f)
    }
}

fn main() {
    let buf = Buf { bytes: [0; 3] };
    let caught = std::panic::catch_unwind(|| {
        let held = buf.metadata(true).map(|held| held.of.bytes.len());
        println!("{buf} {held:?} {}", buf.metadata(false).is_none());
    });
    println!("caught {}", caught.is_err());
    println!("{buf}");
}
