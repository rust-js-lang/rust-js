// `cycle()` of a `Vec`'s or an array's items it owns, whose clone or drop
// shows: std's `Cycle::new` clones all it has left at once, its first
// round the items themselves, moved, never cloned, and each round after a
// clone of all of them again, the one it ends dropped; dropped, it drops
// both it holds, what it started with first. The plain build cloned each
// item as it gave it, its first round too, and refused one with a
// destructor (#1583).
#[derive(Debug)]
struct Loud(u32);

impl Clone for Loud {
    fn clone(&self) -> Loud {
        println!("clone {}", self.0);
        Loud(self.0 + 10)
    }
}

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[derive(Debug)]
struct Shown(u32);

impl Clone for Shown {
    fn clone(&self) -> Shown {
        println!("shown clone {}", self.0);
        Shown(self.0 + 10)
    }
}

#[derive(Clone, Debug)]
struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        println!("drop guard");
    }
}

fn main() {
    let mut it = vec![Loud(1), Loud(2)].into_iter().cycle();
    println!("made");
    for _ in 0..5 {
        println!("{:?}", it.next());
    }
    drop(it);

    println!("-- array");
    let taken: Vec<Loud> = [Loud(3)].into_iter().cycle().take(3).collect();
    println!("{taken:?}");
    drop(taken);

    println!("-- stepped first");
    let mut v = vec![Loud(4), Loud(5), Loud(6)].into_iter();
    let first = v.next();
    println!("{first:?}");
    let mut c = v.cycle();
    println!("{:?} {:?} {:?}", c.next(), c.next(), c.next());
    drop(c);
    drop(first);

    println!("-- none");
    let mut e = Vec::<Loud>::new().into_iter().cycle();
    println!("{:?}", e.next());

    println!("-- clone without drop");
    println!("{:?}", [Shown(7)].into_iter().cycle().take(2).collect::<Vec<_>>());

    println!("-- drop without clone");
    println!("{}", vec![Guard].into_iter().cycle().take(2).count());

    println!("-- copied and borrowed");
    println!("{:?}", vec![1, 2].into_iter().cycle().take(5).collect::<Vec<_>>());
    let held = vec![Loud(8)];
    println!("{:?}", held.iter().cycle().take(2).collect::<Vec<_>>());
}
