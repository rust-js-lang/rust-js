// Clones of what's returned as `impl Trait + Clone`, held: in a closure's
// capture, a struct's field, a tuple, a `Vec` and an `Option`, and through
// generic code, each its own value (#1109, #1211).
trait Stack {
    fn push(&mut self, n: i32);
    fn len(&self) -> usize;
}

#[derive(Clone)]
struct Numbers {
    items: Vec<i32>,
}

impl Stack for Numbers {
    fn push(&mut self, n: i32) {
        self.items.push(n);
    }

    fn len(&self) -> usize {
        self.items.len()
    }
}

fn stack() -> impl Stack + Clone {
    Numbers { items: vec![] }
}

fn counter() -> impl FnMut() -> i32 + Clone {
    let mut count = 0;
    move || {
        count += 1;
        count
    }
}

#[derive(Clone)]
struct Held<T>(T, u8);

fn cloned<T: Clone>(t: &T) -> T {
    t.clone()
}

fn main() {
    // Captured by a closure that's cloned.
    let mut s = stack();
    s.push(1);
    let show = move || s.len();
    let show2 = show.clone();
    println!("{} {}", show(), show2());
    // In a struct, a tuple, a `Vec`, an `Option`.
    let mut held = Held(stack(), 1);
    let copy = held.clone();
    held.0.push(2);
    println!("{} {}", held.0.len(), copy.0.len());
    let mut pair = (stack(), counter());
    let pair2 = pair.clone();
    pair.0.push(3);
    println!("{} {} {}", pair.0.len(), pair2.0.len(), (pair.1)());
    let mut many = vec![stack(), stack()];
    let many2 = many.clone();
    many[0].push(4);
    println!("{} {}", many[0].len(), many2[0].len());
    let mut maybe = Some(counter());
    let maybe2 = maybe.clone();
    println!("{} {}", maybe.as_mut().unwrap()(), maybe2.map(|mut f| f()).unwrap());
    // Through generic code.
    let mut t = counter();
    t();
    let mut u = cloned(&t);
    println!("{} {}", t(), u());
    let mut st = stack();
    let st2 = cloned(&st);
    st.push(5);
    println!("{} {}", st.len(), st2.len());
}
