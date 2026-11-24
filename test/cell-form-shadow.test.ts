// `CellForm`'s shadow (#2490, Campaign 4 phase 1): of each `RefCell`
// borrow, the form the compile takes beside the one the proof gives from
// MIR, alone and with the crate's bodies' borrow summaries, and what
// defeated each; no output changed (ADR 1199's `cell-form` kind).
import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildCompiler, compiler, fixture, shadowLines } from "./support";

beforeAll(buildCompiler, 600_000);

/** Of `source`, compiled once, each borrow's line by its function: `today alone summed
 * shape of defeat-alone defeat-summed`. */
const compiled = new Map<string, Record<string, string[]>>();
function formsOf(source: string): Record<string, string[]> {
  const known = compiled.get(source);
  if (known) return known;
  const dir = fixture("cell-form-shadow");
  const input = join(dir, "case.rs");
  const log = join(dir, "shadow.txt");
  writeFileSync(input, source);
  writeFileSync(log, "");
  const out = runSync([compiler, "--shadow", `cell-form=${log}`, input, "-o", join(dir, "case.js"), "--", "--edition=2024", "--crate-type=bin"], dir, 60_000);
  expect(out.code, out.stderr).toBe(0);
  const forms: Record<string, string[]> = {};
  for (const line of shadowLines(log, "cell-form").filter((l) => l.startsWith("site\t"))) {
    const [, today, alone, summed, shape, of, , , defeatAlone, defeatSummed, fn] = line.split("\t");
    (forms[fn!] ??= []).push([today, alone, summed, shape, of, defeatAlone, defeatSummed].join(" "));
  }
  // And each field used whole, how, `whole`'s.
  forms["whole"] = shadowLines(log, "cell-form")
    .filter((l) => l.startsWith("whole\t"))
    .map((l) => l.split("\t").slice(1, 3).join(" "));
  for (const fn of Object.keys(forms)) forms[fn]!.sort();
  expect(readFileSync(join(dir, "case.js"), "utf8").length).toBeGreaterThan(0);
  compiled.set(source, forms);
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
// reads it through the cell's own `Clone`, or lent to a function, has every
// borrow counted (design point 6); another struct's field, never used
// whole, stays quiet.
test("a field a derive reads or a function is lent whole is counted at every borrow", () => {
  const forms = formsOf(`use std::cell::RefCell;

#[derive(Clone, Debug)]
struct Node {
    kids: RefCell<Vec<u32>>,
}

struct Plain {
    kids: RefCell<Vec<u32>>,
}

struct Lent {
    kids: RefCell<Vec<u32>>,
}

fn count(c: &RefCell<Vec<u32>>) -> usize {
    c.borrow().len()
}

struct Kept {
    kids: RefCell<Vec<u32>>,
}

fn same(c: &RefCell<Vec<u32>>, d: &RefCell<Vec<u32>>) -> bool {
    std::ptr::eq(c, d)
}

fn add_lent(n: &Lent, k: u32) {
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
}
`);
  expect(forms["add"]).toEqual(["unchecked counted counted method param-field whole-use whole-use"]);
  expect(forms["add_plain"]).toEqual(["unchecked quiet quiet method param-field - -"]);
  // Lent whole, `&l.kids`, to what isn't its own borrow.
  expect(forms["add_lent"]).toEqual([expect.stringMatching(/ counted counted method param-field whole-use whole-use$/)]);
  // A lend to a parameter only ever borrowed, `count`'s, is told apart from
  // a derive's read and from a lend to what reads its address: the first
  // whole-use proof's measure (#2490).
  expect(forms["whole"]).toEqual(["Kept::kids used", "Lent::kids lent-to-borrows", "Node::kids used"]);
});
