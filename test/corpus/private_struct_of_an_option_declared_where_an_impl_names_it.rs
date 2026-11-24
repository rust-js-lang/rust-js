// A private struct whose field is an `Option`, named first by an impl of
// another module's trait for it: its declaration, written unexported where
// that impl names it (ADR 0385), asks how its `Option` field holds its
// payload in the struct's own context. It asked the module's, the crate
// root's, which is no item's: a rustc ICE, `generics_of: unexpected node
// kind Crate` (#2533).
struct Info {
    source: Option<u32>,
}

mod observe {
    pub struct Seen {
        pub count: u32,
    }

    pub trait Observe {
        fn observe(&self, seen: &mut Seen) -> String;
    }
}

impl observe::Observe for Info {
    fn observe(&self, seen: &mut observe::Seen) -> String {
        seen.count += 1;
        format!("{:?} after {}", self.source, seen.count)
    }
}

fn main() {
    use observe::Observe;
    let mut seen = observe::Seen { count: 0 };
    println!("{}", Info { source: Some(3) }.observe(&mut seen));
    println!("{}", Info { source: None }.observe(&mut seen));
}
