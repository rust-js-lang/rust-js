// `CellForm`'s shadow (#2490, Campaign 4 phase 1): of each `RefCell`
// borrow, the form the compile takes beside the one the proof gives from
// MIR, alone and with the crate's bodies' borrow summaries, and what
// defeated each; no output changed (ADR 1199's `cell-form` kind).
import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildCompiler, buildReact, compiler, fixture, shadowLines, target } from "./support";

beforeAll(() => {
  buildCompiler();
  buildReact();
}, 600_000);

/** What a crate that calls bindings is compiled with: js's and webapi's. */
const withBindings = ["--extern", `js=${join(target, "libjs.rmeta")}`, "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "-L", target];

/** Of `source`, compiled once, each borrow's line by its function: `today alone summed
 * shape of defeat-alone defeat-summed`, and, where `whole` is asked, the
 * function whose call defeated it and the ADR 0262 differences its range
 * is passed by, `by exceptions`. */
const compiled = new Map<string, Record<string, string[]>>();
function formsOf(source: string, { bindings = false, whole = false } = {}): Record<string, string[]> {
  const key = `${bindings} ${whole} ${source}`;
  const known = compiled.get(key);
  if (known) return known;
  const dir = fixture("cell-form-shadow");
  const input = join(dir, "case.rs");
  const log = join(dir, "shadow.txt");
  writeFileSync(input, source);
  writeFileSync(log, "");
  const out = runSync(
    [compiler, "--shadow", `cell-form=${log}`, input, "-o", join(dir, "case.js"), "--", "--edition=2024", `--crate-type=${bindings ? "lib" : "bin"}`, ...(bindings ? withBindings : [])],
    dir,
    60_000,
  );
  expect(out.code, out.stderr).toBe(0);
  const forms: Record<string, string[]> = {};
  for (const line of shadowLines(log, "cell-form").filter((l) => l.startsWith("site\t"))) {
    const [, today, alone, summed, shape, of, , , defeatAlone, defeatSummed, fn, , by, exceptions] = line.split("\t");
    const fields = [today, alone, summed, shape, of, defeatAlone, defeatSummed, ...(whole ? [by, exceptions] : [])];
    (forms[fn!] ??= []).push(fields.join(" "));
  }
  // And each field used whole, how, `whole`'s.
  forms["whole"] = shadowLines(log, "cell-form")
    .filter((l) => l.startsWith("whole\t"))
    .map((l) => l.split("\t").slice(1, 3).join(" "));
  for (const fn of Object.keys(forms)) forms[fn]!.sort();
  expect(readFileSync(join(dir, "case.js"), "utf8").length).toBeGreaterThan(0);
  compiled.set(key, forms);
  return forms;
}

const program = `use std::cell::RefCell;

struct Log {
    lines: RefCell<Vec<u32>>,
    other: RefCell<u32>,
}

fn total(v: &[u32]) -> u32 {
    v.iter().sum()
}

fn note(log: &Log, n: u32) {
    log.lines.borrow_mut().push(n);
}

fn summed(log: &Log) -> u32 {
    let lines = log.lines.borrow();
    total(&lines)
}

fn bumped(log: &Log) -> u32 {
    let lines = log.lines.borrow();
    bump(log);
    lines.len() as u32
}

fn bump(log: &Log) {
    *log.other.borrow_mut() += 1;
}

fn reentered(log: &Log) -> usize {
    let lines = log.lines.borrow();
    note(log, 9);
    lines.len()
}

fn then_summed(log: &Log) -> u32 {
    log.lines.borrow_mut().push(4);
    total(&[1, 2])
}

fn both(log: &Log) -> usize {
    let a = log.lines.borrow();
    let b = log.lines.borrow_mut();
    a.len() + b.len()
}

fn tried(log: &Log) -> bool {
    log.lines.try_borrow_mut().is_ok()
}

fn main() {
    let log = Log { lines: RefCell::new(vec![1, 2]), other: RefCell::new(0) };
    note(&log, 3);
    println!("{} {} {} {}", summed(&log), bumped(&log), then_summed(&log), tried(&log));
    println!("{}", std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| reentered(&log))).is_err());
    println!("{}", std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| both(&log))).is_err());
}
`;

test("a borrow for a moment of std's method is proved quiet alone", () => {
  expect(formsOf(program)["note"]).toEqual(["momentary quiet quiet method param-field - -"]);
});

test("a guard held across the crate's function the summaries prove quiet, alone counted", () => {
  const forms = formsOf(program);
  expect(forms["summed"]).toEqual(["momentary counted quiet let param-field crate-fn -"]);
  // `bump` borrows another field's cell, of another type: no borrow of it
  // meets the guard's.
  expect(forms["bumped"]).toEqual(["momentary counted quiet let param-field crate-fn -"]);
  // A field that holds its value, never lent nor held where it's asked, a
  // handle on it, unchecked (ADRs 0362, 0863).
  expect(forms["bump"]).toEqual(["unchecked quiet quiet write param-field - -"]);
});

test("a guard's range ends where it's dropped: what runs after asks nothing of it", () => {
  expect(formsOf(program)["then_summed"]).toEqual(["momentary quiet quiet method param-field - -"]);
});

test("a guard held across what borrows its cell again stays counted", () => {
  expect(formsOf(program)["reentered"]).toEqual(["counted counted counted let param-field crate-fn crate-fn"]);
  // Of two borrows of one field, the first's range holds the second.
  expect(formsOf(program)["both"]).toEqual([expect.stringMatching(/ counted counted let param-field second-borrow second-borrow$/), expect.stringMatching(/ let param-field - -$/)]);
});

test("a try_borrow stays counted by design", () => {
  expect(formsOf(program)["tried"]).toEqual(["counted counted counted method param-field try-borrow try-borrow"]);
});

// A field some body uses whole, by a derive rust-js writes itself, which
// reads it through the cell's own `Clone`, or lent to a function, keeps its
// cell and its flag, each borrow checked, where the proof alone can't pass
// the use (design point 6). With the summaries it's unchecked where every
// borrow of its type is proved quiet (proof B) or each use is a lend to a
// parameter only ever borrowed (proof A); checked, not counted, where
// neither holds, its range quiet. Another struct's field, never used whole,
// stays quiet.
test("a field a derive reads or a function is lent whole is checked at each borrow, unchecked where proof A or B passes it", () => {
  const forms = formsOf(`use std::cell::RefCell;

#[derive(Clone, Debug)]
struct Node {
    kids: RefCell<Vec<u32>>,
}

#[derive(Clone)]
struct Loud {
    items: RefCell<Vec<u8>>,
}

fn poke(l: &Loud) {
    l.items.borrow_mut().push(1);
}

fn held(l: &Loud) -> usize {
    let items = l.items.borrow();
    poke(l);
    items.len()
}

fn add_loud(l: &Loud) {
    l.items.borrow_mut().push(2);
}

struct Swapped {
    kids: RefCell<Vec<u32>>,
}

struct Plain {
    kids: RefCell<Vec<u32>>,
}

struct Lent {
    kids: RefCell<Vec<u8>>,
}

fn count(c: &RefCell<Vec<u8>>) -> usize {
    c.borrow().len()
}

struct Kept {
    kids: RefCell<Vec<u32>>,
}

fn same(c: &RefCell<Vec<u32>>, d: &RefCell<Vec<u32>>) -> bool {
    std::ptr::eq(c, d)
}

fn add_lent(n: &Lent, k: u8) {
    n.kids.borrow_mut().push(k);
}

fn add(n: &Node, k: u32) {
    n.kids.borrow_mut().push(k);
}

fn add_plain(n: &Plain, k: u32) {
    n.kids.borrow_mut().push(k);
}

fn main() {
    let n = Node { kids: RefCell::new(vec![]) };
    add(&n, 1);
    println!("{:?}", n.clone());
    let p = Plain { kids: RefCell::new(vec![]) };
    add_plain(&p, 2);
    println!("{}", p.kids.borrow().len());
    let l = Lent { kids: RefCell::new(vec![]) };
    add_lent(&l, 3);
    println!("{}", count(&l.kids));
    let k = Kept { kids: RefCell::new(vec![]) };
    k.kids.borrow_mut().push(4);
    println!("{}", same(&k.kids, &k.kids));
    let loud = Loud { items: RefCell::new(vec![]) };
    add_loud(&loud);
    let swapped = Swapped { kids: RefCell::new(vec![5]) };
    swapped.kids.swap(&RefCell::new(vec![6]));
    println!("{:?}", swapped.kids.borrow());
    println!("{}", std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| held(&loud.clone()))).is_err());
}
`);
  // No borrow of a `RefCell<Vec<u32>>` is loud: proof B.
  expect(forms["add"]).toEqual(["unchecked momentary quiet method param-field whole-use -"]);
  expect(forms["add_plain"]).toEqual(["unchecked quiet quiet method param-field - -"]);
  // Lent whole, `&l.kids`, to `count`, which only borrows it: proof A, of
  // a type \`held\` makes loud.
  expect(forms["add_lent"]).toEqual([expect.stringMatching(/ momentary quiet method param-field whole-use -$/)]);
  // `held`'s guard of a `RefCell<Vec<u8>>` is held while `poke` borrows
  // one, so neither proof holds: checked, never counted, its range quiet.
  expect(forms["add_loud"]).toEqual([expect.stringMatching(/ momentary momentary method param-field whole-use whole-use$/)]);
  // A lend to a parameter only ever borrowed, `count`'s, is told apart from
  // a derive's read and from a lend to what reads its address: the first
  // whole-use proof's measure (#2490).
  // A cell's own \`swap\` is its borrow, not a whole use (#659's rule).
  expect(forms["whole"]).toEqual(["Kept::kids used", "Lent::kids lent-to-borrows", "Loud::items used", "Node::kids used"]);
});

// Tu's decisions (ADR 0863, amended; ADR 0262's differences 39 and 44),
// which the proof keeps as today's output does, each site they pass named
// so, never a proof: a guard that's only a call's receiver is held while
// the call runs, not while its other arguments do, which a binding's or a
// library's code runs; a guard read only for a binding's argument, while
// it's read, not while the binding runs. And a binding's method of what a
// builtin made, which calls back into nothing, passed by the receiver the
// proof finds in its call's THIR.
test("a binding run while a guard's held is passed by Tu's exceptions, named, or by what it's given", () => {
  const forms = formsOf(
    `use std::cell::RefCell;

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.show")]
fn show(value: &js::Unknown) -> String {
    let _ = value;
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.prepare")]
fn prepare(nonce: Option<&str>) {
    let _ = nonce;
    unreachable!()
}

pub struct Segment {
    pub chunks: RefCell<Vec<String>>,
    pub nonce: RefCell<Option<String>>,
    pub counts: RefCell<Vec<u32>>,
}

pub fn push_text(segment: &Segment, text: &str) {
    segment.chunks.borrow_mut().push(text.to_owned());
}

pub fn push_value(segment: &Segment, value: &js::Unknown) {
    segment.chunks.borrow_mut().push(show(value));
}

pub fn push_either(segment: &Segment, value: &js::Unknown, plain: bool) {
    segment.chunks.borrow_mut().push(if plain { String::from("x") } else { show(value) });
}

pub fn push_matched(segment: &Segment, value: &js::Unknown, plain: Option<&str>) {
    segment.chunks.borrow_mut().push(match plain {
        Some(plain) => plain.to_owned(),
        None => show(value),
    });
}

pub fn held_value(segment: &Segment, value: &js::Unknown) {
    let mut chunks = segment.chunks.borrow_mut();
    chunks.push(show(value));
}

pub fn prepared(segment: &Segment) {
    prepare(segment.nonce.borrow().as_deref());
}

pub fn seen(segment: &Segment, key: &str) {
    let map: &'static js::Map<str, u32> = js::map::new(None);
    let mut counts = segment.counts.borrow_mut();
    counts.push(map.get(key).unwrap_or(0));
}
`,
    { bindings: true, whole: true },
  );
  expect(forms["push_text"]).toEqual([expect.stringMatching(/ quiet quiet method param-field - - - -$/)]);
  // `show(value)` runs before the push, the guard its receiver alone: 39.
  expect(forms["push_value"]).toEqual([expect.stringMatching(/ quiet quiet method param-field - - - 39$/)]);
  // Through either arm of an \`if\`, to the one push.
  expect(forms["push_either"]).toEqual([expect.stringMatching(/ quiet quiet method param-field - - - 39$/)]);
  expect(forms["push_matched"]).toEqual([expect.stringMatching(/ quiet quiet method param-field - - - 39$/)]);
  // A guard a \`let\` holds is held while \`show\` runs, which may call back.
  expect(forms["held_value"]).toEqual([expect.stringMatching(/ counted counted let param-field unseen unseen show -$/)]);
  // What the guard guards, read for the binding's argument: 44.
  expect(forms["prepared"]).toEqual([expect.stringMatching(/ quiet quiet method param-field - - - 44$/)]);
  // A \`Map\` a builtin made, at hand: its \`get\` calls back into nothing.
  expect(forms["seen"]).toEqual([expect.stringMatching(/ quiet quiet let param-field - - - -$/)]);
});

// A try_borrow is counted by design, but where nothing can find its cell
// borrowed: no borrow of its type's guard is held where one may be asked
// (ADR 0863's "a RefCell no borrow of which can meet another"). Two fields'
// places are two cells, whichever objects they're of. A type parameter's
// value dropped while a guard's held runs what each caller's instance's
// destructor does (#1321), and a thread-local's \`with_borrow_mut\` holds its
// cell while its closure runs, which may drop what borrows.
test("a try_borrow nothing can contest, another field's borrow and a type parameter's drop", () => {
  const forms = formsOf(`use std::cell::RefCell;
use std::rc::Rc;

struct Tally {
    count: RefCell<u32>,
    other: RefCell<u32>,
    names: RefCell<Vec<String>>,
}

fn bump(t: &Tally) -> bool {
    if let Ok(mut c) = t.count.try_borrow_mut() {
        *c += 1;
        return true;
    }
    false
}

fn both(t: &Tally) {
    let count = t.count.borrow();
    *t.other.borrow_mut() += *count;
}

thread_local! {
    static QUEUE: RefCell<String> = const { RefCell::new(String::new()) };
    static ASKING: RefCell<Vec<Asks>> = const { RefCell::new(Vec::new()) };
}

fn peek() -> bool {
    QUEUE.with(|q| q.try_borrow().is_ok())
}

fn queued() -> bool {
    QUEUE.with_borrow_mut(|q| {
        q.push('a');
        peek()
    })
}

fn hold<T>(c: &RefCell<Vec<T>>, f: &dyn Fn()) {
    let _held = c.borrow();
    f();
}

fn push_plain<T>(it: T, counts: &RefCell<Vec<u16>>) {
    counts.borrow_mut().push({
        let _it = it;
        1
    });
}

fn drop_rc(r: Rc<Asks>, counts: &RefCell<Vec<u8>>) {
    counts.borrow_mut().push({
        let _r = r;
        1
    });
}

fn cleared(counts: &RefCell<Vec<u8>>) -> usize {
    let seen = counts.borrow();
    ASKING.with_borrow_mut(|v| v.clear());
    seen.len()
}

struct Asks(Rc<RefCell<Vec<u8>>>);

impl Drop for Asks {
    fn drop(&mut self) {
        println!("free {}", self.0.try_borrow().is_ok());
    }
}

fn push_after<T>(it: T, counts: &RefCell<Vec<u8>>) {
    counts.borrow_mut().push({
        let _it = it;
        1
    });
}

fn main() {
    let t = Tally { count: RefCell::new(0), other: RefCell::new(0), names: RefCell::new(vec![]) };
    println!("{}", bump(&t));
    let shown: &dyn Fn(&Tally) = &|t| both(t);
    shown(&t);
    t.names.borrow_mut().push(String::new());
    let counts = Rc::new(RefCell::new(vec![]));
    push_after(Asks(Rc::clone(&counts)), &counts);
    drop_rc(Rc::new(Asks(Rc::clone(&counts))), &counts);
    push_plain(String::from("x"), &RefCell::new(vec![]));
    println!("{}", queued());
    let bytes = RefCell::new(vec![1i64]);
    hold(&bytes, &|| println!("{}", bytes.try_borrow_mut().is_err()));
    ASKING.with_borrow_mut(|v| v.push(Asks(Rc::clone(&counts))));
    println!("{}", cleared(&counts));
}
`);
  expect(forms["bump"]).toEqual([expect.stringMatching(/ quiet if-let param-field - -$/)]);
  // \`count\`'s guard is held while \`other\`, of its type, is borrowed:
  // another field's cell.
  expect(forms["both"]).toContainEqual(expect.stringMatching(/ quiet quiet let param-field - -$/));
  // \`Asks\` dropped while the push's guard is held asks of its cell, a
  // type parameter's value or what an \`Rc\` holds.
  expect(forms["push_after"]).toEqual([expect.stringMatching(/ counted method param destructor destructor$/)]);
  // Of each instance its callers make, a \`String\`, which asks nothing.
  expect(forms["push_plain"]).toEqual([expect.stringMatching(/ quiet method param \S+ -$/)]);
  expect(forms["drop_rc"]).toEqual([expect.stringMatching(/ counted method param destructor destructor$/)]);
  expect(forms["<Asks as std::ops::Drop>::drop"]).toEqual([expect.stringMatching(/ counted method \S+ \S+ try-borrow$/)]);
  // \`with_borrow_mut\` holds \`QUEUE\`'s cell while its closure peeks.
  expect(forms["peek::{closure#0}"]).toEqual([expect.stringMatching(/ counted method \S+ \S+ try-borrow$/)]);
  // A generic body's guard, of each instance's cell, held while \`f\` runs.
  expect(forms["main::{closure#1}"]).toEqual([expect.stringMatching(/ counted method \S+ \S+ try-borrow$/)]);
  // The closure \`with_borrow_mut\` runs drops what borrows \`counts\`.
  expect(forms["cleared"]).toEqual([expect.stringMatching(/ counted counted let param std-runs-crate std-runs-crate$/)]);
});

// A class's flattened state used whole, moved out by `into_inner`, has no
// flag to check and is never counted (ADR 1044): its whole use is its
// declaration's to refuse or keep, so it defeats its borrows only alone,
// never with the summaries, and the borrow keeps its declared form (Tu,
// #2490's flip).
test("a flattened state's whole use defeats its borrows alone, never with the summaries", () => {
  const forms = formsOf(`use std::cell::RefCell;

#[derive(Debug)]
#[cfg_attr(rust_js, rust_js::tag = "status")]
pub enum Status {
    #[cfg_attr(rust_js, rust_js::name = "pending")]
    Pending { tries: u32 },
    #[cfg_attr(rust_js, rust_js::name = "halted")]
    Halted,
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct Task {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub state: RefCell<Status>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Task {
    pub fn new(state: Status) -> Self {
        Task { state: RefCell::new(state) }
    }
}

fn halt(task: &Task) {
    *task.state.borrow_mut() = Status::Halted;
}

fn main() {
    let task = Task::new(Status::Pending { tries: 1 });
    halt(&task);
    println!("{:?}", task.state.into_inner());
}
`);
  expect(forms["whole"]).toEqual(["Task::state used"]);
  expect(forms["halt"]).toEqual(["declared counted quiet write param-field whole-use -"]);
});
