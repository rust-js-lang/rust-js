//@ library-differs
// `main` calls into another module, which calls back into this one's
// `make`, which uses a generic impl declared after `main`: the impl's
// cache must be made before `main` runs, as everything its module makes as
// it loads is, though `main`'s own reads stop at the other module (#2527).
struct Wrap<T>(T);

fn make() -> String {
    let w = Wrap(String::from("x"));
    m::shown(&w)
}

fn main() {
    t::run();
}

impl<T: std::fmt::Debug> m::Show for Wrap<T> {
    fn show(&self) -> String {
        format!("{:?}", self.0)
    }
}

mod t {
    pub fn run() {
        println!("{}", super::make());
    }
}

mod m {
    pub trait Show {
        fn show(&self) -> String;
    }

    pub fn shown<S: Show>(s: &S) -> String {
        s.show()
    }
}
