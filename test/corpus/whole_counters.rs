// A number a counter of `f64`s gives is whole, whose JS text is Rust's
// (rd537, ADR 0994, amended): a field made a whole number and only ever set
// one more or one less, as React's Flight reply client counts its parts'
// ids, `let nextPartId = 1; .. formFieldPrefix + blobId`; and a function
// that gives one. A field written any other way, a half, `-0.0`, lent
// mutably, past 2^53, or another crate's to write; a `Cell` field lent, to a
// function, to an array or by a function's result; one of a struct a
// constant holds, whose value rustc evaluates; and a function's of `-0.0`,
// keep Rust's text of any `f64`.
use std::cell::Cell;

struct Reply {
    prefix: String,
    next_id: Cell<f64>,
    pending: Cell<f64>,
    rows: f64,
}

impl Reply {
    fn take_id(&self) -> f64 {
        let id = self.next_id.get();
        self.next_id.set(id + 1.0);
        id
    }

    fn settle(&self) -> f64 {
        self.pending.set(self.pending.get() - 1.0);
        self.pending.get()
    }
}

fn next_of(reply: &Reply) -> f64 {
    reply.take_id()
}

struct Halves {
    at: Cell<f64>,
}

struct Signed {
    at: Cell<f64>,
}

struct Big {
    at: Cell<f64>,
}

fn unsigned_zero() -> f64 {
    -0.0
}

struct Lent {
    n: f64,
}

fn bump(n: &mut f64) {
    *n += 1e21;
}

pub struct Open {
    pub n: f64,
}

struct Handed {
    at: Cell<f64>,
}

fn cell_of(handed: &Handed) -> &Cell<f64> {
    &handed.at
}

struct Given {
    at: Cell<f64>,
}

fn set_negative_zero(cell: &Cell<f64>) {
    cell.set(-0.0);
}

struct Repeated {
    at: Cell<f64>,
}

struct Fixed {
    n: f64,
}

trait Zero {
    const ZERO: Self;
}

struct Assoc {
    n: f64,
}

impl Zero for Assoc {
    const ZERO: Assoc = Assoc { n: -0.0 };
}

fn zero<T: Zero>() -> T {
    T::ZERO
}

struct Promoted {
    n: f64,
}

fn promoted() -> &'static Promoted {
    &Promoted { n: 1e21 }
}

struct Inline {
    n: f64,
}

static SIGNED: Fixed = Fixed { n: -0.0 };
const HUGE: Fixed = Fixed { n: 1e21 };

fn names(prefix: &str, count: u32) -> Vec<String> {
    let mut reply = Reply {
        prefix: prefix.to_string(),
        next_id: Cell::new(1.0),
        pending: Cell::new(0.0),
        rows: 0.0,
    };
    let mut out = Vec::new();
    for _ in 0..count {
        let blob_id = reply.take_id();
        out.push(reply.prefix.clone() + &blob_id.to_string());
        let stream_id = next_of(&reply);
        out.push(reply.prefix.clone() + "S" + &stream_id.to_string());
        reply.rows += 1.0;
        out.push(format!("{}:{}", reply.rows, reply.settle()));
    }
    reply.pending.set(reply.pending.get() + 1.0);
    out.push(reply.pending.get().to_string());
    out
}

fn others() -> Vec<String> {
    let halves = Halves { at: Cell::new(1.0) };
    halves.at.set(halves.at.get() + 0.5);
    let signed = Signed { at: Cell::new(0.0) };
    signed.at.set(-0.0);
    let mut lent = Lent { n: 2.0 };
    bump(&mut lent.n);
    let big = Big { at: Cell::new(1.0) };
    big.at.set(1e21);
    let open = Open { n: 3.0 };
    let handed = Handed { at: Cell::new(1.0) };
    cell_of(&handed).set(-0.0);
    let handed_at = handed.at.get();
    let given = Given { at: Cell::new(1.0) };
    set_negative_zero(&given.at);
    let given_at = given.at.get();
    let repeated = Repeated { at: Cell::new(1.0) };
    let cells = [&repeated.at; 2];
    cells[1].set(-0.0);
    let repeated_at = repeated.at.get();
    let halves_at = halves.at.get();
    let signed_at = signed.at.get();
    let lent_n = lent.n;
    let big_at = big.at.get();
    let open_n = open.n;
    let unsigned = unsigned_zero();
    let signed_n = SIGNED.n;
    let huge_n = HUGE.n;
    let assoc: Assoc = zero();
    let assoc_n = assoc.n;
    let promoted_n = promoted().n;
    let inline_n = const { Inline { n: -0.0 } }.n;
    vec![
        format!("{halves_at}/{halves_at}"),
        format!("{signed_at}/{signed_at}"),
        format!("{lent_n}/{lent_n}"),
        format!("{big_at}/{big_at}"),
        format!("{open_n}/{open_n}"),
        format!("{handed_at}/{handed_at}"),
        format!("{given_at}/{given_at}"),
        format!("{repeated_at}/{repeated_at}"),
        format!("{signed_n}/{signed_n}"),
        format!("{huge_n}/{huge_n}"),
        format!("{assoc_n}/{assoc_n}"),
        format!("{promoted_n}/{promoted_n}"),
        format!("{inline_n}/{inline_n}"),
        "f".to_string() + &unsigned.to_string(),
    ]
}

fn sums(prefix: &str, i: u32, j: u32) -> String {
    prefix.to_string() + &(i + j).to_string() + &(i * j).to_string()
}

fn main() {
    println!("{}", sums("p", 1, 2));
    println!("{:?}", names("p", 3));
    println!("{:?}", others());
}
