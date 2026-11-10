// What a trait method gives behind an `impl Trait`, held by generic code
// and dropped there: its drop is its impl's dictionary's, `$dropHidden`,
// as an associated type's is, `$dropItem`; of std's compiled struct, its
// own; of an impl whose value drops nothing, none. In a library too,
// whose consumers' impls give it (#1296).
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

trait Source {
    fn hidden(n: u32) -> impl DoubleEndedIterator<Item = u32>;
    fn loud(name: &'static str) -> impl Sized;
}

struct Tens;

impl Source for Tens {
    fn hidden(n: u32) -> impl DoubleEndedIterator<Item = u32> {
        let guard = Loud("hidden");
        (0..n).map(move |x| {
            let _ = &guard;
            x * 10
        })
    }
    fn loud(name: &'static str) -> impl Sized {
        Loud(name)
    }
}

struct Quiet;

impl Source for Quiet {
    fn hidden(n: u32) -> impl DoubleEndedIterator<Item = u32> {
        0..n
    }
    fn loud(_: &'static str) -> impl Sized {}
}

fn last_of<S: Source>() -> Option<u32> {
    let mut it = S::hidden(3);
    let last = it.next_back();
    println!("stepped");
    last
}

fn held<S: Source>() {
    let _kept = S::loud("kept");
    println!("holding");
}

fn main() {
    println!("{:?}", last_of::<Tens>());
    println!("{:?}", last_of::<Quiet>());
    held::<Tens>();
    held::<Quiet>();
}
