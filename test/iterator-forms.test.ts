// One meaning per iterator (ADR 0908): a corpus case compiled with every
// iterator adapter std compiles as its struct, none rewritten to the JS
// chain the recognizers make in place (`--std-iterators`), runs as native
// Rust does. That build is what each rewrite must agree with: a case it
// gets wrong is std's struct lowered wrong, or an adapter std doesn't
// compile yet, which `differs` lists, so the list only shrinks.
//
//   case.rs ─┬─ rustc ──► native ──────────────────────────────┐
//            └─ rust-js --std-iterators ──► node ──────────────┴─► the same?
//
//   bun test test/iterator-forms.test.ts                                     # a few cases
//   ITERATOR_FORMS=corpus RUST_JS_CASES=1/6 bun test test/iterator-forms.test.ts   # the corpus, a sixth of it

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";

import { cases, corpus, directives } from "./corpus-cases";
import { agree, compileJs, runJs, runNative, runtimes, show } from "./programs";
import { buildCompiler, caseShard, fixture } from "./support";
import { record, runKey, shared, verified } from "./verified-runs";

beforeAll(() => {
  buildCompiler();
});

// Of the suite's own run, these: each held and made-in-place adapters std
// compiles, stepped from either end, sized and dropped; folded through
// the crate's own, and taken by a `&mut` (ADR 0912); and one `differs`
// lists, which only the build of std's structs alone gets wrong, so a
// build that isn't one fails.
const few = [
  "std_rev.rs",
  "copied_clone_of_its_own.rs",
  "std_inspect.rs",
  "std_flatten.rs",
  "std_flat_map.rs",
  "held_adapters_by_ref.rs",
  "adapter_drops.rs",
  "own_fold_and_nth.rs",
  "by_ref_steps.rs",
  "held_skip_take.rs",
  "skip_take_own_nth.rs",
  "chain_sides_own_steps.rs",
  "rev_of_lazy_chains.rs",
  "range_hints_given.rs",
  "rev_own_back_methods.rs",
  "held_back_folds_by_try_rfold.rs",
  "skip_try_fold.rs",
  "generic_collect_own_fold.rs",
  // The matrices of #492-#496 and #505, and what a skip, a fuse, a
  // peekable and a chain take of the crate's own `last` (#522).
  "generic_own_last.rs",
  "skip_folded_by_consumers.rs",
  "last_takes_its_source.rs",
  "own_count_through_adapters.rs",
  "drop_order_through_take_adapters.rs",
  "skip_of_short_take_drops.rs",
  "cow_texts_joined.rs",
  "scan_object_state.rs",
  "scan_lazy_drops.rs",
  "flat_map_drops.rs",
  "held_enumerate.rs",
  "copied_structs_apart.rs",
  "associated_double_ended.rs",
  "back_steps.rs",
  "generic_accumulations.rs",
  "collected_counts_ask_room.rs",
  "heap_pop_of_a_param.rs",
  "escape_iterators.rs",
  "held_zips.rs",
  "zip_from_back.rs",
  "zip_into_mut_slice.rs",
  "copied_default_own_params.rs",
  "result_given_as_items.rs",
  "range_steps_anywhere.rs",
  "cloned_chains_and_cycles.rs",
  "lazy_chains.rs",
  // Std's `flatten` dropping its front by an impl's dictionary's drop, which
  // may be none; a held `Rev`'s fold of an inclusive range, its `try_rfold`;
  // and a stage's panic, the accumulator std's fold closure holds dropped
  // before what's left, through each fold, `rfold`, `try_fold` and
  // collect std's structs pass it on to, and what an `rfold` takes dropped
  // as it unwinds (#423).
  "flatten_from_its_back.rs",
  "generic_folds_unwind.rs",
  "own_folds_unwind.rs",
  "range_inclusive_exhausted.rs",
  "panic_in_own_next_drops.rs",
  "panic_in_own_rfold_drops.rs",
  "panic_mid_fold_drops.rs",
  "panic_mid_iteration_drops.rs",
  "panic_part_way_drops.rs",
  "passed_item_drops_that_panic.rs",
  // A compiled struct's close, its fields dropped past one whose drop
  // panics (#677).
  "adapter_drop_past_a_panic.rs",
  // A reversed `Vec`'s `take(n).count()`, what it passes dropped as one
  // slice, past a panic too (#670).
  "rev_take_count_drops_a_slice.rs",
  // What a `Vec`'s `nth` and `nth_back` pass, one slice, and a `Drain`'s
  // and a B-tree's, each, given generic code and std's compiled `StepBy`
  // and `Skip` (#668, #703, #704).
  "nth_passed_drops_unwind.rs",
  "nth_passes_a_slice_or_each.rs",
  // A `skip()` of a `Peekable` stepped from its back, by its length (#706).
  "skip_of_a_peekable_from_back.rs",
  // A `Vec`'s `IntoIter::last`, its back taken, then what's left dropped as
  // one slice, asked by std's compiled `Skip`, `Chain`, `Fuse` and
  // `Peekable` (#648).
  "vec_last_takes_its_back.rs",
  // What `last`'s fold has in hand as a later drop panics, a `filter`'s
  // discarded item's or a `take`'s source's, leaked, not dropped (#648).
  "last_fold_gives_back_what_it_kept.rs",
  // An adapter's closure a fold by value moves, dropped before what its
  // source has left, by std's compiled fold too (#710).
  "fold_drops_moved_function_first.rs",
  // A consumer's own closure, dropped before what its fold leaves (#734).
  "consumer_closure_dropped_first.rs",
  // And as a panic in it unwinds, a closure a std call is given (#719).
  "closure_folded_by_value_panics.rs",
  // A `Vec`'s collect of what isn't `TrustedLen`: its first item by
  // `next`, then its hint asked, which may panic (#724, #660).
  "collect_closes_what_its_hint_panics.rs",
  // And its hint asked again each time the `Vec` is full (#728).
  "collect_asks_hint_as_it_grows.rs",
  // A `cloned()` std's compiled `Zip` reads by index, std's compiled
  // `Cloned`'s `__iterator_get_unchecked`, or steps by `next` beside a
  // `chain()` (#727).
  "adapters_own_work_that_panics.rs",
  // A flatten's `count`, each inner iterator's own, by std's compiled
  // `Flatten::count` and `FlatMap::count` (#736).
  "flatten_count_takes_each_inner_count.rs",
  // A `&mut` of the crate's iterator, std's `IntoIterator` of every
  // iterator, as rustc selects it (#742).
  "mut_ref_iterators_into_iter.rs",
];
const which = process.env.ITERATOR_FORMS;
if (which !== undefined && which !== "corpus")
  throw new Error(
    `ITERATOR_FORMS=${which}: \`corpus\`, or unset for a few cases`,
  );
const chosen = which === "corpus" ? caseShard(cases) : few;

/** The cases whose std-struct build doesn't run as native Rust yet, each
 * with why: what's left to do, which only shrinks. One that runs as Rust
 * does is taken off. */
const differs: Record<string, string> = {};

test("the cases a run of a few names are the corpus's, and each listed difference is one", () => {
  for (const name of [...few, ...Object.keys(differs)])
    expect(cases).toContain(name);
});

for (const name of chosen.filter((name) => cases.includes(name))) {
  const file = join(corpus, name);
  const want = directives(readFileSync(file, "utf8"));
  // What doesn't run as Rust does yet, or doesn't run, has no form to
  // check; a malformed directive is the corpus's to report.
  if (
    typeof want === "string" ||
    want.kind === "compile-fail" ||
    want.kind === "ignore-rust-js"
  )
    continue;
  test(`${basename(name, ".rs")} with std's iterator structs`, () => {
    const dir = fixture(`forms-${basename(name, ".rs")}`);
    const native = runNative(file, dir, want.edition, want.native);
    if (typeof native === "string") throw new Error(native);
    const compiled = compileJs(file, dir, want.edition, false, {}, [
      "--std-iterators",
    ]);
    let problem = "";
    if (!("js" in compiled)) {
      problem = `rust-js failed: ${compiled.error.split("\n").slice(0, 6).join("\n")}`;
    } else {
      const js = readFileSync(compiled.js, "utf8");
      const key = runKey([
        "std iterators",
        js,
        native.bytes.stdout,
        native.bytes.stderr,
        show(native.outcome),
        shared(),
      ]);
      if (!verified(key)) {
        const [runtime, cmd] = runtimes[0];
        const ran = runJs(cmd, compiled.js, dir, `${runtime}-std-iterators`);
        if (agree(ran, native))
          record(key, `${name} with std's iterator structs`);
        else
          problem = `${show(ran.outcome)}, printing\n${ran.stdout.slice(0, 400)}\nwhere native Rust ended ${show(native.outcome)}, printing\n${native.stdout.slice(0, 400)}`;
      }
    }
    if (name in differs)
      expect(
        problem,
        `${name} runs as Rust does now: take it off \`differs\``,
      ).not.toBe("");
    else expect(problem).toBe("");
  }, 300_000);
}

// The build of std's structs is that build: of a chain the recognizers make
// in place, `v.iter().map(..).filter(..)`, std's own `Map` and `Filter`,
// not JS's array methods, or the cases above check those again.
test("a build of std's iterator structs makes no adapter in place", () => {
  const dir = fixture("forms-in-place");
  const file = join(dir, "main.rs");
  writeFileSync(
    file,
    `fn main() {
    let v = [1, 2, 3];
    let doubled: Vec<i32> = v.iter().map(|x| x * 2).filter(|x| *x > 2).collect();
    println!("{doubled:?}");
}
`,
  );
  const compiled = compileJs(file, dir, "2024", false, {}, ["--std-iterators"]);
  if (!("js" in compiled)) throw new Error(compiled.error);
  const js = readFileSync(compiled.js, "utf8");
  expect(js).toContain('from "@rust-js/std/iter.js"');
  expect(js).not.toContain(".map(");
});

// A `take(n).rev()` of a `Vec` of destructors, which the build that
// rewrites adapters refuses (a `Take` it can't step from its back), so no
// corpus case has it: what it passes drops as std's `Take::rfold` drops it,
// by its source's `nth_back(len - n - 1)`, then what that gives, where a
// consumer that folds it takes it, a `String` collected of `char`s; and
// by `nth_back(len - n)`, one slice, where one steps it by `next`, a `for`
// loop or a `String` of `String`s; a panic in that slice's drop, the rest
// of it dropped, then what's left; and in generic code, a `Vec` collected
// by its fold where the caller's iterator is `TrustedLen`, by `next` where
// it isn't, a `skip`'s (#670); and through a `peekable()`, or a `fuse()`
// under a `skip`, whose `rfold` or `try_rfold` is its source's, of a
// `String` of `String`s too, its first by `next` and the rest folded (#697).
test("a take's rev, consumed by fold or by next, drops what it passes as std's does", () => {
  const dir = fixture("forms-take-rev");
  const file = join(dir, "main.rs");
  writeFileSync(
    file,
    `use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};

thread_local! {
    static BOOM: Cell<u32> = const { Cell::new(0) };
}

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == BOOM.get() {
            BOOM.set(0);
            panic!("drop {}", self.0);
        }
    }
}

fn five() -> Vec<D> {
    (1..=5).map(D).collect()
}

fn digit(d: D) -> char {
    char::from_digit(d.0, 10).unwrap()
}

fn taken<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: I, n: usize) -> Vec<u32> {
    src.take(n).rev().map(|d| d.0).collect()
}

fn taken_digits<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: I) -> String {
    src.take(2).rev().map(digit).collect()
}

fn taken_set<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: I) -> usize {
    src.take(2).rev().map(|d| d.0).collect::<std::collections::HashSet<_>>().len()
}

fn taken_peeked<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: I) -> Vec<u32> {
    src.take(3).peekable().rev().map(|d| d.0).collect()
}

fn main() {
    println!("generic vec {:?}", taken(five().into_iter(), 2));
    println!("generic vec of a peekable {:?}", taken_peeked(five().into_iter()));
    println!("generic vec of a skip {:?}", taken(five().into_iter().skip(1), 2));
    println!("generic chars {}", taken_digits(five().into_iter()));
    println!("generic set {}", taken_set(five().into_iter()));
    let s: String = five().into_iter().take(3).rev().map(digit).collect();
    println!("chars {s}");
    let s: String = five().into_iter().take(3).rev().map(|d| d.0.to_string()).collect();
    println!("strings {s}");
    let mut s = String::new();
    for d in five().into_iter().take(3).rev() {
        s.push(digit(d));
    }
    println!("for {s}");
    let s = five().into_iter().take(3).rev().fold(String::new(), |mut s, d| {
        s.push(digit(d));
        s
    });
    println!("fold {s}");
    let s: String = five().into_iter().take(3).peekable().rev().map(digit).collect();
    println!("peekable chars {s}");
    let s: String = five().into_iter().take(3).fuse().skip(1).rev().map(digit).collect();
    println!("fuse skip chars {s}");
    let s: String = five().into_iter().take(3).peekable().skip(1).rev().map(digit).collect();
    println!("peekable skip chars {s}");
    let s: String = five().into_iter().take(3).fuse().skip(1).rev().map(|d| d.0.to_string()).collect();
    println!("fuse skip strings {s}");
    let s: String = five().into_iter().take(3).peekable().skip(1).rev().map(|d| d.0.to_string()).collect();
    println!("peekable skip strings {s}");
    BOOM.set(4);
    let r = catch_unwind(AssertUnwindSafe(|| {
        five().into_iter().take(2).rev().map(digit).collect::<String>()
    }));
    println!("chars panics {:?}", r.ok());
}
`,
  );
  const native = runNative(file, dir, "2024");
  if (typeof native === "string") throw new Error(native);
  const compiled = compileJs(file, dir, "2024", false, {}, ["--std-iterators"]);
  if (!("js" in compiled)) throw new Error(compiled.error);
  const ran = runJs(runtimes[0][1], compiled.js, dir, "take-rev");
  expect(ran.stdout).toBe(native.stdout);
  expect(agree(ran, native)).toBe(true);
}, 300_000);

// A consumer's own closure that holds what has a destructor, given by value
// to a fold of what the build that rewrites adapters can't step from its
// back, a `skip()`, a `chain()` or a `filter()` reversed: dropped before what
// its fold leaves, as std's `fold(self, init, f)` drops `f` first, a
// `chain()`'s before its second side's rest (#734); and a `step_by()`'s
// `rfold`'s, once (#753).
test("a consumer's closure folded over a reversed adapter drops before what's left", () => {
  const dir = fixture("forms-consumer-ends");
  const file = join(dir, "main.rs");
  writeFileSync(
    file,
    `struct D(&'static str);

impl Drop for D {
    fn drop(&mut self) {
        print!("drop {} ", self.0);
    }
}

struct It {
    left: u32,
    _held: D,
}

impl Iterator for It {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        (self.left > 0).then(|| {
            self.left -= 1;
            self.left
        })
    }
}

impl DoubleEndedIterator for It {
    fn next_back(&mut self) -> Option<u32> {
        self.next()
    }
}

fn it(left: u32, name: &'static str) -> It {
    It { left, _held: D(name) }
}

fn zero(_guard: &D) -> u32 {
    0
}

fn main() {
    let g = D("f");
    let n = vec![D("x"), D("y"), D("z")].into_iter().skip(1).rev().fold(0, move |a, _| a + 1 + zero(&g));
    println!("| {n}");
    let g = D("f");
    let n = it(2, "it").chain(it(1, "it2")).rev().fold(0, move |a, x| a + x + zero(&g));
    println!("| {n}");
    let g = D("f");
    let n = it(3, "it").filter(|&x| x > 0).rev().fold(0, move |a, x| a + x + zero(&g));
    println!("| {n}");
    let g = D("f");
    let n = (1..6u32).step_by(2).rfold(0, move |a, x| a + x + zero(&g));
    println!("| {n}");
}
`,
  );
  const native = runNative(file, dir, "2024");
  if (typeof native === "string") throw new Error(native);
  const compiled = compileJs(file, dir, "2024", false, {}, ["--std-iterators"]);
  if (!("js" in compiled)) throw new Error(compiled.error);
  const ran = runJs(runtimes[0][1], compiled.js, dir, "consumer-ends");
  expect(ran.stdout).toBe(native.stdout);
  expect(agree(ran, native)).toBe(true);
}, 300_000);
