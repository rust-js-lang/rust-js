// React's `entangledTransitionTypes`, an exported `let` that starts `null`
// and is filled where it's first needed: a thread-local's `Option`, which
// its module's `with_borrow_mut` fills in place, `get_or_insert_with`,
// `Rc::clone` of what it fills, and `get_or_insert_default`, is its
// `export let` as one only set is, and another module reads what it holds
// (ADR 0270, amended).

mod transition {
    use std::cell::RefCell;
    use std::rc::Rc;

    thread_local! {
        #[cfg_attr(rust_js, rust_js::nullable)]
        pub static ENTANGLED: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
        pub static SHARED: RefCell<Option<Rc<Vec<u32>>>> = const { RefCell::new(None) };
        pub static DEPTH: RefCell<Option<u32>> = const { RefCell::new(None) };
    }

    pub fn entangle(types: &[String]) {
        ENTANGLED.with_borrow_mut(|entangled| {
            let queued = entangled.get_or_insert_with(Vec::new);
            for t in types {
                if !queued.contains(t) {
                    queued.push(t.clone());
                }
            }
        });
    }

    pub fn shared() -> Rc<Vec<u32>> {
        SHARED.with_borrow_mut(|shared| Rc::clone(shared.get_or_insert_with(|| Rc::new(vec![1, 2]))))
    }

    pub fn deeper() -> u32 {
        DEPTH.with_borrow_mut(|depth| {
            let depth = depth.get_or_insert_default();
            *depth += 1;
            *depth
        })
    }

    pub fn clear() {
        ENTANGLED.set(None);
        DEPTH.set(None);
    }
}

mod reader {
    pub fn told() -> String {
        let entangled = super::transition::ENTANGLED.with_borrow(|entangled| entangled.clone());
        let depth = super::transition::DEPTH.with_borrow(|depth| *depth);
        format!("{entangled:?} {depth:?}")
    }
}

fn main() {
    println!("{}", reader::told());
    transition::entangle(&["a".to_string(), "b".to_string()]);
    transition::entangle(&["b".to_string(), "c".to_string()]);
    println!("{} {}", reader::told(), transition::deeper());
    println!("{} {}", transition::deeper(), reader::told());
    let first = transition::shared();
    let again = transition::shared();
    println!("{first:?} {again:?}");
    transition::clear();
    println!("{}", reader::told());
    transition::entangle(&["d".to_string()]);
    println!("{} {}", reader::told(), transition::deeper());
}
