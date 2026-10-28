// What an adapter's function holds with a destructor is dropped with the
// adapter, after its source, as Rust drops its fields: consumed to its
// end, stepped part of the way, or held and dropped where its scope ends
// (ADRs 0485, 0991).
struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let v = vec![1u32, 2, 3, 4];
    let a = Noisy(1);
    let seen = v
        .iter()
        .inspect(move |x| {
            let k = &a;
            println!("see {x} {}", k.0);
        })
        .count();
    println!("{seen} counted");
    let b = Noisy(2);
    let first = v
        .iter()
        .skip_while(move |x| {
            let k = &b;
            **x <= k.0
        })
        .next();
    println!("{first:?} skipped");
    let c = Noisy(3);
    let flat: Vec<u32> = v
        .iter()
        .flat_map(move |x| {
            let k = &c;
            vec![*x; k.0 as usize - 2]
        })
        .collect();
    println!("{flat:?} flattened");
    let d = Noisy(4);
    let kept: Vec<u32> = v
        .iter()
        .filter_map(move |x| {
            let k = &d;
            (*x != k.0).then_some(x * 10)
        })
        .collect();
    println!("{kept:?} kept");
    let e = Noisy(5);
    let small: Vec<&u32> = v
        .iter()
        .take_while(move |x| {
            let k = &e;
            **x + k.0 < 8
        })
        .collect();
    println!("{small:?} taken");
    let f = Noisy(6);
    let mut held = v.iter().map_while(move |x| {
        let k = &f;
        (*x < k.0).then_some(x + k.0)
    });
    println!("{:?} held", held.next());
    println!("end");
}
