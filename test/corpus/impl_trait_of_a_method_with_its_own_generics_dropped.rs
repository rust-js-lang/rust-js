// A trait method's `impl Trait` of generics of its own its drop can't read,
// a type's bound by `Copy`, `loud<U: Copy>`, or a constant's, `counted<const
// M: u32>`, held by generic code and dropped there: the impl's dictionary
// carries its drop, one of every instance, as of an associated type, so
// what it holds is dropped as Rust drops it, where it was refused in a
// crate whose types may have a destructor (#1575).

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

trait Source {
    fn loud<U: Copy>(n: u32, tag: U) -> impl DoubleEndedIterator<Item = (u32, U)>;
    fn counted<const M: u32>(n: u32) -> impl DoubleEndedIterator<Item = u32>;
}

struct Tens;

impl Source for Tens {
    fn loud<U: Copy>(n: u32, tag: U) -> impl DoubleEndedIterator<Item = (u32, U)> {
        let guard = Loud("tens");
        (0..n).map(move |x| {
            let _ = &guard;
            (x * 10, tag)
        })
    }

    fn counted<const M: u32>(n: u32) -> impl DoubleEndedIterator<Item = u32> {
        let guard = Loud("counted");
        (0..n).map(move |x| {
            let _ = &guard;
            x * M
        })
    }
}

fn held<S: Source>() {
    let it = S::loud(3, 'a');
    let back: Vec<(u32, char)> = it.rev().collect();
    println!("{back:?}");
    let mut counted = S::counted::<5>(4);
    println!("{:?} {:?}", counted.next_back(), counted.next());
    println!("before the end");
}

fn main() {
    held::<Tens>();
    println!("after");
}
