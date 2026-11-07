// A struct's `Option` field of a value with a destructor is dropped where
// it's `Some`. Its `None` is tested as the field holds it: `null` for a
// `#[rust_js::nullable]` field, `undefined` for a plain one. So an empty
// nullable field, `null`, drops nothing, where a test of `!== undefined`
// would drop `null`'s value (#887). Native and JS agree.
struct Noisy(u8);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub struct Slot {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub held: Option<Noisy>,
    pub plain: Option<Noisy>,
    pub name: u8,
}

fn main() {
    let empty = Slot {
        held: None,
        plain: None,
        name: 1,
    };
    let full = Slot {
        held: Some(Noisy(2)),
        plain: Some(Noisy(3)),
        name: 2,
    };
    println!("{} {}", empty.name, full.name);
    drop(empty);
    drop(full);
    println!("end");
}
