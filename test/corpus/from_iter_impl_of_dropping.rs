// A crate's `FromIterator` impl that collects what it's given into a
// `Vec`, `Req { cs: Vec::from_iter(iter) }`, semver's `VersionReq` of its
// `Comparator`s (#794): its `from_iter::<I>` is std's trait's generic
// method, so a call of it through `FromIterator`'s dictionary, a generic
// `collect`, gives the facts of its own `I` it asks, its iterator's
// in-place buffer, as a crate trait's own generic method's are (ADR 0466).
// Each item dropped where Rust drops it, a step's panic's too (#800).

struct D(i32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Req {
    cs: Vec<D>,
}

impl FromIterator<D> for Req {
    fn from_iter<I: IntoIterator<Item = D>>(iter: I) -> Self {
        Req { cs: Vec::from_iter(iter) }
    }
}

fn collected<T: FromIterator<D>>(n: i32) -> T {
    (1..=n).map(D).collect()
}

// Of a `Vec`'s own iterator, which std collects in place: its buffer's
// facts given through the dictionary.
fn collected_vec<T: FromIterator<D>>(v: Vec<D>) -> T {
    v.into_iter().collect()
}

// Of its own `I`, whose `IntoIter`'s facts it's given by its caller.
fn relayed<T: FromIterator<D>, I: IntoIterator<Item = D>>(iter: I) -> T {
    iter.into_iter().collect()
}

fn main() {
    let direct: Req = vec![D(1), D(2)].into_iter().collect();
    println!("direct {}", direct.cs.len());
    drop(direct);
    let generic: Req = collected(3);
    println!("generic {}", generic.cs.iter().map(|d| d.0).sum::<i32>());
    drop(generic);
    let from_vec: Req = collected_vec(vec![D(4), D(5)]);
    println!("from vec {}", from_vec.cs.len());
    drop(from_vec);
    let from_relay: Req = relayed(vec![D(6), D(7)]);
    println!("relayed {}", from_relay.cs.len());
    drop(from_relay);
    let caught = std::panic::catch_unwind(|| {
        let _: Req = (1..=3)
            .map(|n| if n == 3 { panic!("at {n}") } else { D(n) })
            .collect();
    });
    println!("{}", caught.is_err());
    let through = std::panic::catch_unwind(|| {
        let _: Req = collected_vec(vec![D(8), D(9), D(10)].into_iter().map(|d| if d.0 == 10 { panic!("at 10") } else { d }).collect());
    });
    println!("{}", through.is_err());
}
