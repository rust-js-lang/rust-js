// Two traits implemented for `&mut Bag`, each with a method that points
// the `&mut` at another `Bag`, `*self = ..`, called through generic code,
// which holds `&mut Bag` as an in-place cell of the `Bag`: the caller's
// `&mut` is repointed, and the `Bag` it pointed at is as it was (#1732).
// `Grow`'s method is the impl's own, `Restart`'s the trait's default, which
// the impl's dictionary copies in.
struct Bag {
    n: u32,
}

trait Grow {
    fn grow(&mut self);
}

impl Grow for &mut Bag {
    fn grow(&mut self) {
        if self.n == 0 {
            *self = Box::leak(Box::new(Bag { n: 100 }));
        }
        self.n += 1;
    }
}

trait Restart: Sized {
    fn fresh() -> Self;

    fn restart(&mut self) {
        *self = Self::fresh();
    }
}

impl Restart for &mut Bag {
    fn fresh() -> Self {
        Box::leak(Box::new(Bag { n: 50 }))
    }
}

fn twice<G: Grow>(g: &mut G) {
    g.grow();
    g.grow();
}

fn restarted<R: Restart>(r: &mut R) {
    r.restart();
}

fn main() {
    let mut bag = Bag { n: 0 };
    let mut b = &mut bag;
    twice(&mut b);
    println!("{}", b.n);
    restarted(&mut b);
    b.n += 1;
    println!("{}", b.n);
    println!("{}", bag.n);

    let mut other = Bag { n: 7 };
    let mut o = &mut other;
    twice(&mut o);
    restarted(&mut o);
    println!("{}", o.n);
    println!("{}", other.n);
}
