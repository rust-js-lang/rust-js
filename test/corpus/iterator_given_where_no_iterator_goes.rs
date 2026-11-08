// The crate's own iterator given for a type parameter bound by neither
// `Iterator` nor `IntoIterator`, `T: Debug`'s or `T: Clone`'s, goes as its
// own value, which that one's dictionaries read: a JS iterator is made of
// it only where generic code steps it (#991).
#[derive(Debug, Clone)]
struct Gen(u32);

impl Iterator for Gen {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            None
        } else {
            self.0 -= 1;
            Some(self.0)
        }
    }
}

impl Drop for Gen {
    fn drop(&mut self) {
        println!("drop gen {}", self.0);
    }
}

fn show<T: std::fmt::Debug>(t: T) -> String {
    format!("{t:?}")
}

fn twice<T: Clone>(t: T) -> (T, T) {
    (t.clone(), t)
}

fn kept<T>(t: T) -> Vec<T> {
    vec![t]
}

fn main() {
    println!("{}", show(Gen(3)));
    let (a, b) = twice(Gen(2));
    println!("{} {}", a.count(), b.0);
    let held = kept(Gen(4));
    println!("{}", held[0].0);
    let total: u32 = kept(Gen(3)).into_iter().flatten().sum();
    println!("{total}");
}
