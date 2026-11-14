// A `Vec`'s `IntoIter` stepped past from its back drops what it passes
// over as one slice, front to back, as `advance_back_by` drops it, of items
// whose drop is drop glue, a struct's that holds a `D`, an `Option<D>`'s, a
// pair's and a `Vec<D>`'s, as of items with a `Drop` of their own (#1854).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        print!("drop {} ", self.0);
    }
}

struct Held(D);

fn helds() -> Vec<Held> {
    (1..=5).map(|n| Held(D(n))).collect()
}

fn options() -> Vec<Option<D>> {
    vec![Some(D(1)), None, Some(D(3)), Some(D(4)), Some(D(5))]
}

fn pairs() -> Vec<(D, D)> {
    (1..=4).map(|n| (D(n), D(n + 10))).collect()
}

fn vecs() -> Vec<Vec<D>> {
    vec![vec![D(1)], vec![D(2), D(12)], vec![], vec![D(4), D(14)]]
}

fn main() {
    let x = helds().into_iter().rev().skip(1).nth(1).map(|h| h.0 .0);
    println!("| held rev skip nth {x:?}");
    let x = helds().into_iter().nth_back(2).map(|h| h.0 .0);
    println!("| held nth_back {x:?}");
    let x = options().into_iter().rev().nth(2).map(|o| o.map(|d| d.0));
    println!("| option rev nth {x:?}");
    let x = pairs().into_iter().rev().skip(2).next().map(|(a, b)| a.0 + b.0);
    println!("| pair rev skip next {x:?}");
    let x = vecs().into_iter().nth_back(1).map(|v| v.len());
    println!("| vec nth_back {x:?}");
    let x = (1..=5).map(D).collect::<Vec<_>>().into_iter().rev().nth(2).map(|d| d.0);
    println!("| d rev nth {x:?}");
}
