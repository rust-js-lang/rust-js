// The crate's own methods named as JS's own, `map`, `at`, `join` and
// `values`, which change what they're given: a read written after the call
// is read after it, as Rust evaluates `println!`'s arguments in order. A
// method so named was taken for JS's own, which only reads, so the read was
// hoisted ahead of its call and saw the state before it (#1663).
struct Grid {
    count: i32,
}

impl Grid {
    fn map(&mut self) -> i32 {
        self.count += 10;
        self.count
    }

    fn at(&mut self, i: i32) -> i32 {
        self.count += i;
        self.count
    }

    fn values(&mut self) -> Vec<i32> {
        self.count *= 2;
        vec![self.count]
    }

    fn twice(&mut self) -> String {
        format!("{} {}", self.map(), self.count)
    }
}

trait Joined {
    fn join(&mut self) -> i32;
}

impl Joined for Grid {
    fn join(&mut self) -> i32 {
        self.count -= 3;
        self.count
    }
}

fn show(g: &mut Grid) {
    println!("{} {:?}", g.map(), (g.count, 1));
    println!("{} {:?}", g.at(5), (g.count, 2));
    println!("{:?} {:?}", g.values(), (g.count, 3));
    println!("{} {:?}", g.join(), (g.count, 4));
    println!("{} {:?}", g.twice(), (g.count, 5));
}

fn main() {
    let mut a = Grid { count: 1 };
    let mut b = Grid { count: 100 };
    show(&mut a);
    show(&mut b);
    show(&mut a);
}
