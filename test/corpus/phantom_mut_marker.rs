// A marker of a borrow, anyhow's `Mut<'a, T>`, whose `PhantomData<&'a mut
// T>` only ties it to the `&mut` it was made of: it holds no value, so no
// `&mut` generic code has, and a call of a generic function that takes or
// gives one is no `&mut` inside what it takes or gives (#516). It was
// refused, "a `&mut ErrorImpl` inside a generic function's parameters".
use std::marker::PhantomData;

#[derive(Debug)]
struct Frame {
    name: String,
    depth: u32,
}

struct Mut<'a, T> {
    at: usize,
    lifetime: PhantomData<&'a mut T>,
}

struct Ref<'a, T> {
    at: usize,
    lifetime: PhantomData<&'a T>,
}

impl<'a, T> Mut<'a, T> {
    fn new(_of: &'a mut T, at: usize) -> Self {
        Mut {
            at,
            lifetime: PhantomData,
        }
    }

    fn by_ref(self) -> Ref<'a, T> {
        Ref {
            at: self.at,
            lifetime: PhantomData,
        }
    }

    fn moved(self, by: usize) -> Mut<'a, T> {
        Mut {
            at: self.at + by,
            lifetime: PhantomData,
        }
    }
}

// Inside an `Option` and a tuple of the result, too.
fn first_of<'a, T>(of: &'a mut T, at: Option<usize>) -> (Option<Mut<'a, T>>, usize) {
    match at {
        Some(at) => (Some(Mut::new(of, at)), at),
        None => (None, 0),
    }
}

fn read<T>(marker: Ref<'_, T>) -> usize {
    marker.at
}

fn main() {
    let mut frame = Frame {
        name: String::from("main"),
        depth: 1,
    };
    let marker = Mut::new(&mut frame, 2).moved(3);
    println!("{}", read(marker.by_ref()));
    frame.depth += 1;
    frame.name.push_str("()");
    let (found, at) = first_of(&mut frame, Some(7));
    println!("{} {}", found.map(|m| m.moved(1).at).unwrap_or(0), at);
    let (none, at) = first_of(&mut frame, None);
    println!("{} {}", none.is_none(), at);
    frame.depth *= 10;
    println!("{frame:?}");
}
