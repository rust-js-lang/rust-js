// A trait's default, copied into its impl, indexing a slice of its own
// type parameter by the crate's `Index<Id> for [T]`, as aho-corasick's
// `Automaton::try_replace_all` does.

#[derive(Clone, Copy)]
struct Id(u32);

impl<T> std::ops::Index<Id> for [T] {
    type Output = T;

    fn index(&self, index: Id) -> &T {
        &self[index.0 as usize]
    }
}

trait Automaton {
    fn ids(&self) -> Vec<Id>;

    fn each<F: FnMut(Id)>(&self, mut f: F) {
        for id in self.ids() {
            f(id);
        }
    }

    fn replace_all<B>(&self, replace_with: &[B]) -> String
    where
        Self: Sized,
        B: AsRef<str>,
    {
        let mut dst = String::new();
        self.each(|id| dst.push_str(replace_with[id].as_ref()));
        dst
    }
}

struct Fixed;

impl Automaton for Fixed {
    fn ids(&self) -> Vec<Id> {
        vec![Id(2), Id(0)]
    }
}

fn main() {
    println!("{}", Fixed.replace_all(&["a", "bb", "ccc"]));
}
