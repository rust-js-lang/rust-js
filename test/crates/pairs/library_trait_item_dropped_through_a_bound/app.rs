//! The library's generic code given its own impl's items and the app's,
//! whose type has a destructor the library never sees.
struct Mine(&'static str);

impl Drop for Mine {
    fn drop(&mut self) {
        println!("mine dropped {}", self.0);
    }
}

struct Own;

impl dep::Source for Own {
    type Named = Mine;

    fn named(&self, tag: &'static str) -> Mine {
        Mine(tag)
    }

    fn hidden(&self, tag: &'static str) -> impl Sized {
        Mine(tag)
    }
}

pub fn main() {
    dep::paths(&dep::Louds);
    dep::paths(&Own);
}
