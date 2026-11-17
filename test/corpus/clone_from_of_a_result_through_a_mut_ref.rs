// `clone_from` of a `Result` written directly, whose `&mut` is the `Result`
// itself (#2028): of two `Ok`s the value's own `clone_from`, a `Vec`'s in
// place or a type's own, of two `Err`s the error's; otherwise a clone
// written over it, what it held dropped. Through a parameter, generic or
// not, a reborrow and a local, of a value whose `&mut` is a box too; and of
// a `Result` moved out of a struct's field, which leaves the struct's new
// value and its other fields as they were.
#[derive(Debug)]
struct Counted(u32);

impl Clone for Counted {
    fn clone(&self) -> Counted {
        println!("clone {}", self.0);
        Counted(self.0)
    }

    fn clone_from(&mut self, source: &Counted) {
        println!("clone_from {} <- {}", self.0, source.0);
        self.0 = source.0;
    }
}

impl Drop for Counted {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

// A value whose `&mut` is a box, not an object, a fieldless enum's: its
// `clone_from` writes it whole, the `Result` holding it written in place.
#[derive(Debug)]
enum Mode {
    Fast,
    Slow,
}

impl Clone for Mode {
    fn clone(&self) -> Mode {
        match self {
            Mode::Fast => Mode::Fast,
            Mode::Slow => Mode::Slow,
        }
    }

    fn clone_from(&mut self, source: &Mode) {
        println!("mode clone_from {self:?} <- {source:?}");
        *self = source.clone();
    }
}

fn assign_mode(dest: &mut Result<Mode, u8>, source: &Result<Mode, u8>) {
    dest.clone_from(source);
}

#[derive(Debug)]
struct Holder {
    result: Result<Vec<Counted>, u8>,
    label: &'static str,
}

fn assign<T: Clone, E: Clone>(dest: &mut Result<T, E>, source: &Result<T, E>) {
    dest.clone_from(source);
}

fn assign_vec(dest: &mut Result<Vec<Counted>, u8>, source: &Result<Vec<Counted>, u8>) {
    dest.clone_from(source);
}

fn main() {
    let mut w: Result<Vec<Counted>, u8> = Ok(vec![Counted(7)]);
    assign(&mut w, &Ok(vec![Counted(3), Counted(1)]));
    println!("generic {w:?}");
    assign_vec(&mut w, &Ok(vec![Counted(4)]));
    println!("concrete {w:?}");
    assign_vec(&mut w, &Err(5));
    println!("to err {w:?}");
    assign_vec(&mut w, &Ok(vec![Counted(6)]));
    println!("to ok {w:?}");

    let mut r: Result<Counted, Counted> = Ok(Counted(10));
    let p = &mut r;
    p.clone_from(&Ok(Counted(11)));
    println!("reborrow {r:?}");
    r.clone_from(&Err(Counted(12)));
    println!("local to err {r:?}");
    r.clone_from(&Err(Counted(13)));
    println!("local err {r:?}");
    assign(&mut r, &Ok(Counted(14)));
    println!("generic own {r:?}");

    let mut m: Result<Mode, u8> = Ok(Mode::Fast);
    assign_mode(&mut m, &Ok(Mode::Slow));
    println!("mode {m:?}");
    assign(&mut m, &Ok(Mode::Fast));
    println!("generic mode {m:?}");

    let mut holder = Holder {
        result: Ok(vec![Counted(20)]),
        label: "kept",
    };
    let mut moved = holder.result;
    assign_vec(&mut moved, &Ok(vec![Counted(21), Counted(22)]));
    println!("moved {moved:?}, label {}", holder.label);
    holder.result = Ok(vec![Counted(23)]);
    assign(&mut moved, &Ok(vec![Counted(24)]));
    println!("holder {holder:?}");
    println!("moved {moved:?}");
}
