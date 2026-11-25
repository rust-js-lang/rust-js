// The iterator rules of the architecture, split from architecture.test.ts at
// its cap (a test file is at most 1500 lines): who gives an iterator's steps,
// which own step an adapter reaches, and who answers whether stepping it may
// panic (ADR 1089).
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { iteratorPaths, touchesIterators } from "../scripts/iteratorSlice";
import { ownerPath, sources } from "../scripts/owners";
import { root } from "./support";
import { compilerRustFiles } from "../scripts/compiler-crates";

const read = (path: string) => readFileSync(join(root, path), "utf8");
// In name order, as macOS lists them and Linux, CI's, doesn't.
const files = (directory: string): string[] => readdirSync(join(root, directory), { withFileTypes: true })
  .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
  .flatMap(entry => entry.isDirectory() ? files(`${directory}/${entry.name}`) : [`${directory}/${entry.name}`]);

// An iterator's steps beside `next`, its `fold`, `nth` and the others std's
// adapters specialize, are given by `$forward` alone (ADR 1089): each one
// passed on by hand forwarded the subset of steps, and of their arguments,
// it was written with, `$mapEnds` no `fold`, `$enumerate` no `unwind`
// (#621, #626).
test("an iterator's steps are given by $forward alone", () => {
  const given = /\.(fold|tryFold|rfold|tryRfold|nth|nthBack|last|count|countItems|advanceBy|advanceBackBy|advance)\s*=\s*(\([^)]*\)\s*=>|\w+\s*=>|function\b|\()/;
  expect(given.test("  it.nth = (k) => step(nth(iterator, k));")).toBe(true);
  const giving = files("crates/rust-js-program/src/runtime")
    .filter((file) => file.endsWith(".js") && file !== "crates/rust-js-program/src/runtime/forward.js")
    .filter((file) => read(file).split("\n").some((line) => !line.trimStart().startsWith("//") && given.test(line)));
  expect(giving).toEqual([]);
});

// An iterator's own facts, `ownNth` and the others (ADR 1092 §4), are
// written by forward.js's `$owns` and `$ownsAs` alone, where the iterator
// is made: each adapter copied its source's by hand, a subset of its own
// choosing (#870).
test("an iterator's own facts are written by $owns alone", () => {
  const writes = /\.own(?:Fold|Rfold|Nth|NthBack|Last|Count)\s*=(?!=)|\[\$ownStep\([^)]*\)\]\s*=(?!=)/;
  expect(writes.test("  if (it.ownNth) closing.ownNth = true;")).toBe(true);
  expect(writes.test("  if (own !== undefined) it[$ownStep(name)] = true;")).toBe(true);
  expect(writes.test("  if (it.ownNth === true) return;")).toBe(false);
  const writing = files("crates/rust-js-program/src/runtime")
    .filter((file) => file.endsWith(".js") && file !== "crates/rust-js-program/src/runtime/forward.js")
    .filter((file) => read(file).split("\n").some((line) => !line.trimStart().startsWith("//") && writes.test(line)));
  expect(writing).toEqual([]);
  expect(read("crates/rust-js-program/src/runtime/forward.js")).toContain("function $ownsAs(adapter, source, ...steps)");
});

// An iterator's lifecycle, open, taking, taken or closed, is one field,
// `$state`, changed by closing.js's transitions alone, `$close`, `$takenBy`
// and `$stateFrom` (ADR 1092 S7): `$closing`'s local `closed`, std's
// struct's `$closed` and a fold's `closed = true` were three marks that
// disagreed, a drop twice or none (#734, #762, #817, #870).
test("an iterator's lifecycle is written by closing.js's transitions alone", () => {
  const writes = /\.\$state\s*=(?!=)|\.\$closed\b|\bclosed\s*=\s*(?:true|false)\b/;
  expect(writes.test("    it.$closed = true;")).toBe(true);
  expect(writes.test("  let closed = false;")).toBe(true);
  expect(writes.test("  if (it.$state === \"open\") return;")).toBe(false);
  const writing = files("crates/rust-js-program/src/runtime")
    .filter((file) => file.endsWith(".js") && file !== "crates/rust-js-program/src/runtime/closing.js")
    .filter((file) => /\$Steps|\$closing|\$stdIterator/.test(read(file)))
    .filter((file) => read(file).split("\n").some((line) => !line.trimStart().startsWith("//") && writes.test(line)));
  expect(writing).toEqual([]);
  const closing = read("crates/rust-js-program/src/runtime/closing.js");
  const transitions = ["function $close(it, drop)", "function $takenBy(it, run, closesItself = false)", "function $stateFrom(target, value)"];
  for (const transition of transitions) expect(closing).toContain(transition);
  const outside = closing.slice(0, closing.indexOf("function $close(it, drop)"));
  expect(outside.split("\n").filter((line) => !line.trimStart().startsWith("//") && writes.test(line))).toEqual([]);
});

// Which of the crate's own steps std's adapters reach is one catalog's,
// `OwnStep::asked_of`, and one query's, `reaches_its_own` (ADR 1089): a
// predicate per step, each with its own list of adapters, treated a type
// parameter and a `&mut` each its own way (#496, #505, #521).
test("which own step an adapter reaches is the OwnStep catalog's alone", () => {
  const lowering = [...files("crates/rust-js-lower/src"), ...files("crates/rust-js-mir/src"), "src/pipeline.rs", ...files("src/pipeline")].map(read).join("\n");
  expect(lowering).not.toMatch(/fn (counts|lasts|folds|nths|rfolds|try_rfolds|nth_backs|may_fold)_its_own\b/);
  expect(read("crates/rust-js-lower/src/recognition/own_steps.rs")).toContain("fn asked_of(self, adapter: &str)");
});

// Whether an iterator is written in Rust, the crate's or a library's, whose
// own steps std's adapters reach, is `Recognition::is_user_iterator`'s alone:
// a site that answered it by `is_local()` beside its own `Iterator` test took
// a library's iterator for std's, and its chain stepped another `nth` than
// std's (#2067), as a maker's own list missed a library's back (#2068).
test("whether an iterator is written in Rust is is_user_iterator's alone", () => {
  const asks = /implements_trait\([^;]*\biterator\b|has_user_impl\(\s*iterator\b|is_user_impl\([^;]*\biterator\b|StdItem::Iterator/;
  const own = (lines: string[]) => lines.flatMap((line, i) =>
    /is_local\(\)/.test(line) && asks.test(lines.slice(Math.max(0, i - 3), i + 4).join("\n")) ? [line.trim()] : []);
  expect(own(["matches!(part.kind(), ty::Adt(adt, _) if adt.did().is_local()) && recognition.implements_trait(part, iterator)"])).toHaveLength(1);
  const deciding = [...files("crates/rust-js-lower/src"), ...files("crates/rust-js-mir/src"), "src/pipeline.rs", ...files("src/pipeline")]
    .filter((file) => file.endsWith(".rs") && file !== "crates/rust-js-lower/src/recognition/iterators.rs")
    .flatMap((file) => own(read(file).split("\n").map((line) => line.replace(/\/\/.*$/, ""))).map((line) => `${file}: ${line}`));
  expect(deciding).toEqual([]);
  expect(read("crates/rust-js-lower/src/iterators/transparent.rs")).toContain("recognition.is_user_iterator(part)");
});

// What an adapter's fold or `nth`, from either end, asks of what it holds is
// that catalog's too, wherever another question follows it: slice_steps.rs
// kept its own list of adapters whose `rfold` asks an `nth_back`, without the
// `peekable()` the catalog had, so generic code's `take(3).peekable().rev()`
// was collected by `next`, dropping 4 before 5 where std drops 5 before 4,
// and the list of those whose `nth` asks one, the catalog's own row (#697).
// A function of the lowering named for a fold or an `nth` names no adapter of
// std's but in own_steps.rs, which asks it of `asked_sources`. Nor does one
// named for a slice: slice_steps.rs's `nth` followed a `Rev` to a bare `Vec`
// alone, not through a `take(n)`'s `nth_back`, and iterators.rs's
// `steps_past_a_slice` listed the adapters whose `next` is their source's,
// without a `take_while()`, so a `step_by` of each was a filter by index that
// dropped back to front where std drops one slice front to back (#2239).
test("which source an adapter's fold or nth asks is the OwnStep catalog's alone", () => {
  const names = "Map|Filter|FilterMap|Take|Skip|Fuse|Peekable|Enumerate|StepBy|Chain|Rev|Inspect|Copied|Cloned|Flatten|FlatMap|SkipWhile|TakeWhile|Scan|MapWhile|Zip";
  const adapter = new RegExp(`"(?:std::iter::)?(${names})"|StdPath::Iter(${names})\\b`);
  const listed: string[] = [];
  for (const file of [...files("crates/rust-js-lower/src"), ...files("crates/rust-js-mir/src"), "src/pipeline.rs", ...files("src/pipeline")].filter((file) => file.endsWith(".rs"))) {
    if (file === "crates/rust-js-lower/src/recognition/own_steps.rs") continue;
    for (const item of read(file).split(/\n(?=\s*(?:pub(?:\([^)]*\))? )?fn \w+)/)) {
      const name = item.match(/fn (\w+)/)?.[1];
      if (name !== undefined && /fold|nth|_a_slice/.test(name) && adapter.test(item)) listed.push(`${file}: fn ${name}`);
    }
  }
  expect(listed).toEqual([]);
  const slices = read("crates/rust-js-lower/src/recognition/slice_steps.rs");
  expect(slices).toContain("self.asked_sources(ty, step)");
  expect(slices).toContain("self.asked_by_next(ty)");
});

// Whether stepping an iterator may panic, which decides what a fold's
// `unwind` and a collect's `$collectDropping` drop, is `steps_may_panic`'s
// alone, of the iterator's type (ADR 1089): its JS's arrows, `chain_may_throw`,
// and its `$closing`s, `closes_a_source`, answered it too, and disagreed
// with the type where a compiled `fuse()` closed nothing of a `Vec`'s rest
// (#653). Neither gate is given the JS it gates.
test("whether an iterator's steps may panic is steps_may_panic's alone", () => {
  const ends = ["collect.rs", "consumers.rs"].map((file) => read(`crates/rust-js-lower/src/iterators/mir_steps/${file}`)).join("\n");
  for (const gate of ["unwinding_collect", "stranded_drop"]) {
    expect(ends, gate).toContain(`fn ${gate}(`);
    const signature = ends.slice(ends.indexOf(`fn ${gate}(`), ends.indexOf(") ->", ends.indexOf(`fn ${gate}(`)));
    expect(signature, gate).not.toContain("Expr");
  }
  const lowering = [...files("crates/rust-js-lower/src"), ...files("crates/rust-js-mir/src"), "src/pipeline.rs", ...files("src/pipeline")].map(read).join("\n");
  expect(lowering).not.toMatch(/fn (chain_may_throw|closes_a_source|callbacks_in)\b/);
});

// An adapter that holds a function with a destructor is `$holding(adapter,
// drop, moved)` alone, which `closing_function` makes by `iterators::holding`,
// the form's one writer, which keeps its adapter's marks (#2069), so its function drops
// where std's fold drops it (#710): `$closing(adapter, (it) => { it.return();
// drop f })`, the form it replaced, dropped it after what a `take` left
// wherever a fold by value moved it. No lowering writes that arrow, and no
// JS the tests pin nor the guide holds it.
test("an adapter that holds a function is $holding alone", () => {
  const lowering = [...files("crates/rust-js-lower/src"), ...files("crates/rust-js-mir/src"), "src/pipeline.rs", ...files("src/pipeline")].filter((file) => file.endsWith(".rs"));
  const making = lowering.filter((file) => read(file).includes('"$holding"'));
  expect(making).toEqual(["crates/rust-js-lower/src/iterators/value.rs", "crates/rust-js-lower/src/iterators.rs"]);
  expect(read("crates/rust-js-lower/src/iterators/value.rs")).toContain('let holding = Expr::call_helper("$holding", [adapter, drop].into_iter().chain(moved).collect());');
  expect(read("crates/rust-js-mir/src/iter.rs")).toContain("rust_js_lower::iterators::holding(");
  expect(lowering.filter((file) => read(file).includes('Expr::arrow(vec!["it".into()]'))).toEqual([]);
  const old = /\(it\) => \{\s*it\.return\(\);/;
  expect(old.test("$closing(take.map(f), (it) => {\n    it.return();\n    drop(g);\n  })")).toBe(true);
  // Not the decisions, which keep what was decided before.
  const pinned = [...files("test").filter((file) => file.endsWith(".js")), ...files("docs/guide")];
  expect(pinned.filter((file) => old.test(read(file)))).toEqual([]);
});

// A fold by value drops what drops with its `f`, `ends`, before what's left
// of what it folds, as std's `fold(self, init, f)` drops `f` first (#710,
// #734): every runtime `fold` and `rfold` that takes an `unwind` takes
// `ends` after it and passes it on or drops it there; one that took
// `(init, f, unwind)` alone lost it, and a consumer's closure dropped after
// what that fold closed.
test("every runtime fold takes what drops with its f", () => {
  const without = /\b(r?fold)\s*(=|:)\s*(\w+(\.\w+)?\s*&&\s*)?\(+\s*init, \w+, unwind\)\s*=>|^\s*r?fold\(init, \w+, unwind\)\s*\{|\(init, \w+, unwind\) => r?fold\(/m;
  expect(without.test("  const fold = (init, f, unwind) => {")).toBe(true);
  expect(without.test("  const fold = (init, f, unwind, ends) => {")).toBe(false);
  const lacking = files("crates/rust-js-program/src/runtime")
    .filter((file) => file.endsWith(".js"))
    .filter((file) => read(file).split("\n").some((line) => !line.trimStart().startsWith("//") && without.test(line)));
  expect(lacking).toEqual([]);
});

// So does every runtime `tryFold` and `tryRfold`, `(init, f, broke, unwind,
// ends)`: a `try_fold` drops its `f` at its end, its caller's `$endsAfter`,
// but an adapter whose first step runs while its frame holds only its
// parameters, `SkipWhile`'s, `Skip`'s, `StepBy`'s and a `take`'s back,
// drops `f` before the accumulator as it unwinds (`$stepHolding`), so each
// takes `ends` and passes it on where std moves `f` into its source's
// (#768). One that took `(init, f, broke, unwind)` alone lost it; the one
// runtime `Chain`'s gives `ends` to the side std moves `f` into (#739).
test("every runtime tryFold takes what drops with its f", () => {
  const without = /\b(try(Fold|Rfold))\s*(=|:)\s*(\w+(\.\w+)?\s*(&&|\?)\s*)?\(+\s*init, \w+, broke, unwind\)\s*=>|^\s*try(Fold|Rfold)\(init, \w+, broke, unwind\)\s*\{|\(init, \w+, broke, unwind\) => try(Fold|Rfold)\(/m;
  expect(without.test("  const tryFold = (init, f, broke, unwind) => {")).toBe(true);
  expect(without.test("  tryFold(init, f, broke, unwind) {")).toBe(true);
  expect(without.test("  const tryFold = (init, f, broke, unwind, ends) => {")).toBe(false);
  const lacking = files("crates/rust-js-program/src/runtime")
    .filter((file) => file.endsWith(".js"))
    .filter((file) => read(file).split("\n").some((line) => !line.trimStart().startsWith("//") && without.test(line)));
  expect(lacking).toEqual([]);
});

// A by-value consumer's own function, where its drop is seen, is dropped by
// `$consume`, which its fold is given (#734): `consumed_function` alone says
// which, `consumed_ends` alone makes its drop, and `dropped_given` skips it.
// No other lowering writes `$consume`, so a consumer that dropped its
// closure after the call, around what the call closed, can't come back.
test("a consumer's closure folded by value is dropped by $consume alone", () => {
  const lowering = [...files("crates/rust-js-lower/src"), ...files("crates/rust-js-mir/src"), "src/pipeline.rs", ...files("src/pipeline")].filter((file) => file.endsWith(".rs"));
  const writing = lowering.filter((file) => read(file).includes('"$consume"'));
  for (const file of writing) expect(read(file), file).toContain("consumed_ends(");
  expect(writing.sort()).toEqual(["crates/rust-js-lower/src/iterators/mir_steps/backs.rs", "crates/rust-js-lower/src/iterators/mir_steps/consumers.rs"]);
  const asking = lowering.filter((file) => read(file).includes("consumed_function("));
  // And `consumed_drop`'s keyed extreme, whose key function moved into its
  // fold makes its consumer close what it takes (#789, #2572).
  expect(asking.sort()).toEqual([
    "crates/rust-js-lower/src/iterators/mir_steps/consumed.rs",
    "crates/rust-js-lower/src/iterators/mir_steps/consumers.rs",
    "crates/rust-js-mir/src/dispatch.rs",
  ]);
  expect(read("crates/rust-js-program/src/runtime/fold_of.js")).toContain("function $consume(it, step, init, f, unwind, ends)");
});

// What the iterators' code is, which runs the iterator matrix's slice in a
// push's check and in check:fast (#731), is `iteratorPaths` alone: each
// module docs/architecture.md says owns a rule of iterators or their steps
// is one of its paths, and each of its paths is a file there is.
test("the iterator matrix's slice runs of each module that owns a rule of iterators", () => {
  const files = sources(root);
  const owners = read("docs/architecture.md")
    .split("\n")
    .filter((line) => line.startsWith("| ") && /iterat|step/i.test(line.split("|")[1] ?? ""))
    .flatMap((line) => [...(line.split("|")[2] ?? "").matchAll(/`([\w/]+(?:\.rs|\.js|\/))`/g)].map((m) => m[1]))
    .map((module) => ownerPath(module!, files))
    .map((module) => (module.endsWith("/") ? `${module}x.rs` : module));
  expect(owners).toContain("crates/rust-js-lower/src/recognition/own_steps.rs");
  expect(owners.filter((module) => !touchesIterators([module]))).toEqual([]);
  const tracked = Bun.spawnSync(["git", "ls-files", "--cached", "--others", "--exclude-standard"], { cwd: root }).stdout.toString().split("\n");
  expect(iteratorPaths.filter((path) => !tracked.some((file) => new Bun.Glob(path).match(file)))).toEqual([]);
  expect(touchesIterators(["crates/rust-js-lower/src/jsx.rs", "docs/guide/README.md"])).toBe(false);
});

// An expression rebuilt of its own kind keeps what its maker marked it,
// `facts: its_facts(e.facts)` (lower/iterators/value.rs): the iterator
// value's marks are the one way a reader past its maker knows what it is
// (ADR 1092, amended). `iterated`, `from_iterator` and tidy's `unarrayed`
// rebuilt a range stepped from its end with `Facts::NONE`, and a `for` over
// `(a..b).rev()` stopped counting down (#730).
test("an expression rebuilt of its own kind keeps its value's marks", () => {
  const rebuilt = /Expr \{\s*facts:\s*(?:js::)?Facts::NONE,\s*kind,/;
  const dropping = [...files("crates/rust-js-lower/src"), ...files("crates/rust-js-mir/src"), "src/pipeline.rs", ...files("src/pipeline")]
    .filter((file) => file.endsWith(".rs") && rebuilt.test(read(file)));
  expect(dropping).toEqual([]);
});

// Std's `Chain` is `$lazyChain` alone, which knows its back where both sides
// know theirs (ADR 0545's #739 amendment, the user's decision): a second,
// `$chainEnds`, had std's back steps and not its front ones, and the two
// disagreed (#739, #752). No runtime file declares another chain, and no
// lowering writes one.
test("std's Chain is $lazyChain alone", () => {
  const chains = /^(?:function\*? |const |let |class )(\$\w*[Cc]hain\w*)\b/gm;
  const declared = files("crates/rust-js-program/src/runtime")
    .filter((file) => file.endsWith(".js"))
    .flatMap((file) => [...read(file).matchAll(chains)].map((m) => m[1]));
  expect(declared).toEqual(["$lazyChain"]);
  const lowering = [...files("crates/rust-js-lower/src"), ...files("crates/rust-js-mir/src"), "src/pipeline.rs", ...files("src/pipeline")].filter((file) => file.endsWith(".rs"));
  expect(lowering.filter((file) => /"\$\w*[Cc]hain\w*"/.test(read(file).replaceAll('"$lazyChain"', "")))).toEqual([]);
});

// What drops many parts drops each in turn, the next though one's drop
// throws, by `$inTurn` alone (ADR 0465's #743 amendment): a close written
// as one `return()` after another stopped at a panicking first part and
// leaked the rest, a chain's `b`, a zip's, a holding map's function (#743,
// #762).
test("an adapter's parts are closed in turn by $inTurn alone", () => {
  const closes = /\.return(\?\.)?\(\);?\s*$/;
  const back = (lines: string[]) =>
    lines.flatMap((line, i) => (i > 0 && closes.test(line) && closes.test(lines[i - 1]) ? [`${i}: ${lines[i - 1].trim()} ${line.trim()}`] : []));
  expect(back(["    a.return?.();", "    b.return?.();"])).toHaveLength(1);
  const by = files("crates/rust-js-program/src/runtime")
    .filter((file) => file.endsWith(".js"))
    .flatMap((file) => back(read(file).split("\n")).map((line) => `${file}:${line}`));
  expect(by).toEqual([]);
  expect(read("crates/rust-js-program/src/runtime/steps.js")).toContain("function $inTurn(...runs)");
});

// What a runtime helper drops as a panic unwinds is `$unwinds(run,
// ...drops)`'s alone, each drop given in std's frame's order, and what a
// frame's end or a drop glue runs in turn is `$inTurn`'s and `$inTurnOf`'s,
// all in steps.js (ADR 1092's #817 amendment): each helper's own `try` kept
// its own order and its own answer of which error goes on, and 40 of them
// drifted (#843, #844, #845, iterators × protocol gap). The helpers below
// aren't an iterator's: what each runs as it throws is its own owner's, the
// panic points of #854 to make one with it.
const ownUnwinding: Record<string, string> = {
  "crates/rust-js-program/src/runtime/borrow.js": "a `RefCell` borrow given back as the call it guards throws",
  "crates/rust-js-program/src/runtime/dedup_by.js": "a `Vec`'s `dedup_by` of a test that may panic, its items kept as std's guard keeps them",
  "crates/rust-js-program/src/runtime/fmt_error.js": "a `fmt::Error` caught to become `write!`'s `Err`, not a drop",
  "crates/rust-js-program/src/runtime/from_json.js": "serde's decoding error caught to become a `Result`, not a drop",
  "crates/rust-js-program/src/runtime/heap_ops.js": "a `BinaryHeap`'s `retain` of a test that may panic",
  "crates/rust-js-program/src/runtime/json_content.js": "serde's decoding error caught to become a `Result`, not a drop",
  "crates/rust-js-program/src/runtime/json_decoder.js": "serde's decoding error caught to become a `Result`, not a drop",
  "crates/rust-js-program/src/runtime/json_flat.js": "serde's decoding error caught to become a `Result`, not a drop",
  "crates/rust-js-program/src/runtime/json_path.js": "serde's decoding error caught to become serde_path_to_error's `Result`, and the reader's place given back as the read ends, not a drop (ADR 1188)",
  "crates/rust-js-program/src/runtime/json_read.js": "JSON.parse's `SyntaxError` caught to become a `Result`, not a drop",
  "crates/rust-js-program/src/runtime/json_reader.js": "serde's decoding error caught to become a `Result`, not a drop",
  "crates/rust-js-program/src/runtime/json_value.js": "serde's encoding error caught to become a `Result`, not a drop",
  "crates/rust-js-program/src/runtime/debug_builders.js": "a `Debug` builder's part's write error caught as std's `result`, and what a part a panic stopped wrote written through `{:#?}`'s adapter before it goes on, not a drop (#1533)",
  "crates/rust-js-program/src/runtime/fs.js": "Node's error of a call of the OS's caught to become std's `io::Error`, a pipe's positioned read or write tried again where it is, and a collected file's close, whose error std's drop ignores, not a drop (#1156)",
  "crates/rust-js-program/src/runtime/json_value_reader.js": "serde's decoding error caught to become a `Result`, not a drop",
  "crates/rust-js-program/src/runtime/map_ops.js": "a map's `retain` of a test that may panic",
  "crates/rust-js-program/src/runtime/print.js": "a line formatted through stdout's: its frame popped, and what's left of the line written, as a panic leaves it, not a drop (#1490)",
  "crates/rust-js-program/src/runtime/pop_if_cell.js": "a `Vec`'s `pop_if` cell written back as its test returns or panics, not a drop (#906)",
  "crates/rust-js-program/src/runtime/rc.js": "`Rc::unwrap_or_clone` of a clone that may panic",
  "crates/rust-js-program/src/runtime/retain.js": "a `Vec`'s `retain` of a test that may panic, its items kept as std's guard keeps them",
  "crates/rust-js-program/src/runtime/should_panic.js": "a `#[should_panic]` test's panic caught, not a drop",
  "crates/rust-js-program/src/runtime/sift_down.js": "a `BinaryHeap`'s sift's hole written back as a comparison panics, as std's `Hole` writes it, not a drop (#901)",
  "crates/rust-js-program/src/runtime/sort_stable.js": "a stable sort's merge, its copy written back where the gap is as a comparison panics, as std's `MergeState` writes it, not a drop (#925)",
  "crates/rust-js-program/src/runtime/sort_unstable.js": "an unstable sort's gaps, and its small sort's merge, written back as a comparison panics, as std's guards write them, not a drop (#908)",
  "crates/rust-js-program/src/runtime/sift_up.js": "a `BinaryHeap`'s sift's hole written back as a comparison panics, as std's `Hole` writes it, not a drop (#901)",
  "crates/rust-js-program/src/runtime/slice_ops.js": "a slice's `is_sorted_by` and `retain_mut` of a function that may panic",
  "crates/rust-js-program/src/runtime/to_json.js": "serde's encoding error caught to become a `Result`, not a drop",
  "crates/rust-js-program/src/runtime/try.js": "`?`'s early return of an `Err` thrown and caught, not a drop",
  "crates/rust-js-program/src/runtime/writer_sink.js": "the sink armed for a writer shown into a writer disarmed as the writer returns or panics, and a writer that never claimed it rust-js's error, not a drop (#1652)",
  "crates/rust-js-program/src/runtime/unwind.js": "`catch_unwind`, the one catch of a panic",
  "crates/rust-js-program/src/runtime/main.js": "a binary's `main` run as std's `lang_start` runs it, its panic ending the process with 101, not a drop (ADR 1184)",
};

test("a runtime helper drops what it holds as a panic unwinds by $unwinds alone", () => {
  const unwinding = /^\s*(\}\s*)?(try\s*\{|catch\s*\(|finally\s*\{)|\}\s*(catch\s*\(|finally\s*\{)/;
  expect(unwinding.test("  } finally {")).toBe(true);
  expect(unwinding.test("    try {")).toBe(true);
  const owners = new Set(["$unwinds", "$dropsUnwinding", "$inTurnOf"]);
  const catching = files("crates/rust-js-program/src/runtime")
    .filter((file) => file.endsWith(".js") && !(file in ownUnwinding))
    .flatMap((file) => {
      let fn = "";
      return read(file)
        .split("\n")
        .flatMap((line, i) => {
          fn = /^function (\$\w+)/.exec(line)?.[1] ?? fn;
          const own = file === "crates/rust-js-program/src/runtime/steps.js" && owners.has(fn);
          return !line.trimStart().startsWith("//") && unwinding.test(line) && !own ? [`${file}:${i + 1} (${fn})`] : [];
        });
    });
  expect(catching).toEqual([]);
  // Each one listed still writes its own, or it's off the list.
  const stale = Object.keys(ownUnwinding).filter((file) => !read(file).split("\n").some((line) => unwinding.test(line)));
  expect(stale).toEqual([]);
  // And the lowering of iterators writes none into the JS: a `skip_while`'s
  // test is the runtime's `$kept` (#817).
  const lowering = [...files("crates/rust-js-lower/src/iterators"), ...files("crates/rust-js-lower/src/iterators/mir_steps")]
    .filter((file) => file.endsWith(".rs"))
    .filter((file) => /StmtKind::Try(Catch)?\(/.test(read(file)));
  expect(lowering).toEqual([]);
});

// A drop that panics as a panic unwinds aborts, as Rust's does (#847), and a
// `finally` can't tell that from a drop on the way on. So what the lowering
// runs as a panic unwinds is guarded by one owner: drops/parts.rs's
// `cleanup`, the one writer of `$cleanup`, and `dropped_after`, the one
// writer of a `finally` of a drop but tidy's scope merges, each of which
// holds only what can't throw (`finally_holds`). Tidy's contract "what
// unwinds is guarded" checks every body it leaves.
test("what the lowering runs as a panic unwinds is guarded by drops/parts.rs alone", () => {
  const guard = "crates/rust-js-lower/src/drops/parts.rs";
  const writes = (pattern: RegExp, allowed: string[]) =>
    [...files("crates/rust-js-lower/src"), ...files("crates/rust-js-mir/src"), "src/pipeline.rs", ...files("src/pipeline"), ...files("crates/rust-js-program/src/prepare")]
      .filter((file) => file.endsWith(".rs") && !allowed.includes(file))
      .flatMap((file) =>
        // A unit test's JS, `#[cfg(test)]`'s, writes nothing the lowering runs.
        read(file)
          .split("#[cfg(test)]")[0]
          .split("\n")
          .filter((line) => !line.trimStart().startsWith("//") && pattern.test(line))
          .map((line) => `${file}: ${line.trim()}`),
      );
  expect(writes(/"\$cleanup"/, [guard])).toEqual([]);
  expect(writes(/StmtKind::Try\([^)]*\)\.at\(/, [guard, "crates/rust-js-mir/src/tidy/unwinds.rs"])).toEqual([]);
  // Each of tidy's merges into a `finally` asks `finally_holds` first.
  const tidy = read("crates/rust-js-mir/src/tidy/unwinds.rs");
  expect(tidy.match(/StmtKind::Try\([^)]*\)\.at\(/g)?.length).toBe(2);
  expect(tidy.match(/finally_holds\)?/g)?.length).toBeGreaterThanOrEqual(3);
  // And the runtime's one cleanup is `$cleanup`'s too.
  expect(read("crates/rust-js-program/src/runtime/steps.js")).toMatch(/function \$dropsUnwinding\(drops\) \{\n  \$cleanup\(/);
});

// An iterator value's facts are written once, by lower/iterators/value.rs's
// `emit()`, and read only through value.rs, which reads the `Known` facts
// `emit()` marks its JS with (ADR 1092, amended, #730 phase 3). A reader that
// took a helper's name, a call's shape or a span for a fact met a value of
// another form and answered wrongly: `.of` (#721), a closure's result judged
// by another call's span (#716, #726), a length off an assumed shape (#745).
// A new fact goes to value.rs: `emit()` marks it, and value.rs gives the
// reader its accessor.
const valueOwner = "crates/rust-js-lower/src/iterators/value.rs";
const owned = (what: string) =>
  `${what} outside value.rs: ${valueOwner} owns an iterator value's form; add a fact \`emit()\` marks, and an accessor there`;
const loweringSources = () => [...files("crates/rust-js-lower/src"), ...files("crates/rust-js-mir/src"), "src/pipeline.rs", ...files("src/pipeline")].filter((file) => file.endsWith(".rs"));
const linesOf = (file: string) => read(file).split("\n").filter((line) => !line.trimStart().startsWith("//"));
const outsideValue = (pattern: RegExp) =>
  loweringSources()
    .filter((file) => file !== valueOwner)
    .flatMap((file) => linesOf(file).filter((line) => pattern.test(line)).map((line) => `${file}: ${line.trim()}`));

test("an iterator value's readers are defined in value.rs alone", () => {
  const readers =
    /\bfn (closed_parts|stepped_form|stepped_value|unlent|is_lent|stepped_range|stepped_from|bounds_read|its_facts|gives_made_in_place)\b/;
  expect([owned("a reader of an iterator value"), outsideValue(readers)]).toEqual([owned("a reader of an iterator value"), []]);
});

test("$iterator, $given, $knowing, $closing, $lent, $rev, $stepBy, $stepByEnds, $repeatingN and $enumerate are written by value.rs alone", () => {
  const written = /(call_helper|Expr::helper)\("\$(iterator|given|knowing|closing|lent|rev|stepBy|stepByEnds|repeatingN|enumerate)"/;
  expect([owned("a write of an iterator value's helper"), outsideValue(written)]).toEqual([owned("a write of an iterator value's helper"), []]);
});

test("no iterator fact is read off a member's name, `.of`, `.at`, `.end`, `.iter` or `.items`, outside value.rs", () => {
  const member = /Member\([^)]*\)\s*if\s+\w+\s*==\s*"(of|at|end|iter|items)"|Member\(_,\s*"(of|at|end|iter|items)"/;
  expect([owned("a member read as an iterator's fact"), outsideValue(member)]).toEqual([owned("a member read as an iterator's fact"), []]);
});

// Whether a value is a `rev()`, an `enumerate()` or a `map()` made in place
// is its maker's mark (`made_in_place`, #730 step 6b): body_queries says which
// calls make one (`in_place_adapters`), and the value carries the mark past
// spills and locals. No answer is keyed by the span of the call that steps
// it: a table of those, or a question of a type and a span, fails here.
test("whether a value is made in place is its maker's mark, never a stepping call's span", () => {
  const spanKeyed = /in_place_adapter_steps|\bsteps_in_place\b|\bmarked_in_place\b|\bcompiled_struct_at\b|\bspill_marked\b/;
  const found = loweringSources().flatMap((file) =>
    linesOf(file)
      .map((line, i) => [line, i] as const)
      .filter(([line]) => spanKeyed.test(line) && !line.trim().startsWith("//"))
      .map(([, i]) => `${file}:${i + 1}`),
  );
  expect(found).toEqual([]);
});

// No iterator helper is read by its name outside value.rs: each reader asks
// value.rs's accessor of the fact `emit()` marked (steps 1 to 11). A new read
// fails here, with the owner to add the accessor to.
test("no iterator helper is read by its name outside value.rs", () => {
  const helpers = "closing|iterator|given|lent|stepBy|stepByEnds|rev|repeatingN|knowing|enumerate";
  const read_ = new RegExp(`Some\\("\\$(${helpers})"`);
  const literal = new RegExp(`"\\$(${helpers})"`, "g");
  const found: Record<string, number> = {};
  for (const file of loweringSources().filter((file) => file !== valueOwner)) {
    for (const line of linesOf(file).filter((line) => read_.test(line))) {
      for (const [, helper] of line.matchAll(literal)) {
        const key = `${file} $${helper}`;
        found[key] = (found[key] ?? 0) + 1;
      }
    }
  }
  expect([owned("a read of an iterator helper by its name"), found]).toEqual([owned("a read of an iterator helper by its name"), {}]);
});

// Whether a value is made knowing its back is its value's fact, back_stepped's
// marks of its maker's MIR identity (ADR 0545, #851), asked where the value
// is made, and as a type's question where no value is in hand
// (`knows_its_back`, `knows_its_length`), never of a value past its maker
// (ADR 1092, amended, #730 step 8). Each reader is listed per file with its
// reason. A new one fails, and so does a gone one until its row goes too.
test("what's stepped from its back is asked where a value is made, or as a type's question", () => {
  const allowed: Record<string, [number, string]> = {
    "crates/rust-js-mir/src/iter.rs": [1, "`cloned_apart_call`, the maker of a map or a filter cloned apart"],
    "crates/rust-js-mir/src/std_calls/iterators.rs": [1, "`mir_iter_std_call`, the maker of a std adapter made in place"],
    "crates/rust-js-lower/src/iterators/mir_steps/backs.rs": [
      2,
      "`back_kept`, the maker of an enumeration it steps from its back, and `rev_called`, a `rev()`'s, of what `rev_items` makes; a type parameter given as it is is made knowing its back whatever's asked of it (#2139)",
    ],
    "crates/rust-js-lower/src/iterators/knowing.rs": [
      10,
      "`reversed_made_knowing` and `reversed_knowing`, a `rev()`'s maker; `knows_its_back` and `knows_its_length`, a type's questions, `knows_its_back`'s of a `str`'s search by a pattern that runs code too (#1501); `hint_known_made_dyn`, a type's question at the coercion that makes a `dyn Iterator` of it (#1476)",
    ],
  };
  const reader = /\.(stepped_from_its_back|reversed_made_knowing)\(/;
  const found: Record<string, number> = {};
  for (const file of loweringSources()) {
    const count = linesOf(file).filter((line) => reader.test(line)).length;
    if (count > 0) found[file] = count;
  }
  const expected = Object.fromEntries(Object.entries(allowed).map(([file, [count]]) => [file, count]));
  expect([owned("a read of what's stepped from its back past a value's maker"), found]).toEqual([
    owned("a read of what's stepped from its back past a value's maker"),
    expected,
  ]);
});

// Whether an adapter is made knowing its back is its value's fact, by the MIR
// identity of what holds it, never its type's (#851): back_stepped gives no
// set of the types stepped from their back, only `typed`, those whose
// question is a type's, a type behind an `impl`, a library's, a generic
// function's of its callers'; and nothing compares a value's answer with a
// type's, as the migration's cross-check did. `counted`, the locals an
// enumeration stepped from its back is made of, is a value's too, by the
// MIR local that holds it (#1354).
test("what's stepped from its back is its value's, no type's set", () => {
  const owner = read("crates/rust-js-lower/src/analysis/back_stepped.rs");
  const struct = owner.slice(owner.search(/pub(?:\([^)]*\))? struct BackStepped<'tcx> \{/));
  const fields = [...struct.slice(0, struct.indexOf("\n}\n")).matchAll(/^    pub(?:\(crate\))? (\w+):/gm)].map((m) => m[1]);
  const signature = owner.slice(owner.indexOf("pub(super) fn back_stepped<'tcx>("));
  const returns = signature.match(/\n\) -> ([^\n]+) \{\n/)?.[1];
  const crossChecks = loweringSources().filter((file) => read(file).includes("BACK_STEPPED_CROSS_CHECK"));
  expect([owned("a type's set of what's stepped from its back"), fields, returns, crossChecks]).toEqual([
    owned("a type's set of what's stepped from its back"),
    // Each but `typed` a value's, by its local: `steps`, what its later
    // uses ask of it, and `steps_read`, the bodies that says of (#1458).
    ["typed", "borrows", "locals", "lengths", "steps", "steps_read"],
    "IndexSet<TypedKey<'tcx>>",
    [],
  ]);
});

// Whether a counted adapter, a `skip(n)`, a `take(n)` or a `step_by(n)`, is
// stepped from its back is back_stepped's `stepped_locals` alone, by the MIR
// identity of what holds it (#814): a temporary lent to a back step, a
// variable's value given to one, a chain's side, what an adapter holds
// beneath, a function's return place. Not a type's question, which another
// value of the type need not share: `typed` records no counted adapter,
// `IterLocals` holds no back-step fact of its locals, and no
// lowering file but back_stepped.rs scans MIR's calls for what steps from
// its back through a `&mut`.
test("a counted adapter is stepped from its back by back_stepped's stepped_locals alone", () => {
  // The types' half of the owner, back_stepped.rs; its locals' half is
  // back_stepped/locals.rs (#1458's split by concept).
  const types = read("crates/rust-js-lower/src/analysis/back_stepped.rs");
  const locals = read("crates/rust-js-mir/src/temporaries.rs");
  const struct = locals.slice(locals.indexOf("pub(super) struct IterLocals {"));
  const fields = [...struct.slice(0, struct.indexOf("\n}\n")).matchAll(/^    (?:pub\(super\) )?(\w+):/gm)].map((m) => m[1]);
  const scans = loweringSources().filter(
    (file) =>
      file !== "crates/rust-js-lower/src/analysis/back_stepped/locals.rs" &&
      read(file).includes("TerminatorKind::Call") &&
      read(file).includes("BorrowKind::Mut") &&
      /steps_from_its_back\(\)|IterMethod::(NextBack|NthBack|RFind)\b/.test(read(file)),
  );
  expect([owned("a second fact of a counted adapter stepped from its back"), types.includes("counted_source("), fields, scans]).toEqual([
    owned("a second fact of a counted adapter stepped from its back"),
    false,
    ["maps", "marks", "once"],
    [],
  ]);
});

// Whether a collect's items drop, by which generic code asks its caller
// what std collects in place, is the lowering's own drops' answer,
// `drops::drops_given`, the analysis's too: `ItemDrops` asked
// `drop_runs`, which says an associated type of no dictionary may drop,
// unsupported, where the lowering's drops say a `G<J::Item>` with a `Drop`
// of its own runs, so it asked nothing and the lowering's collect was
// refused (#737).
test("whether a collect's items drop is asked of the lowering's drops", () => {
  expect(read("crates/rust-js-lower/src/analysis/type_facts.rs")).not.toMatch(/drop_runs|runs_where/);
  const analysis = read("crates/rust-js-lower/src/analysis.rs");
  const made = analysis.slice(analysis.indexOf("let item_drops = type_facts::ItemDrops"));
  expect(made.slice(0, made.indexOf("};"))).toContain("drops: &items_drop");
  expect(analysis).toContain("super::drops::drops_given(");
  // It asks the walk `FnCx::drops` asks, of the drops its function is given.
  const drops = read("crates/rust-js-lower/src/drops.rs");
  const given = drops.slice(drops.indexOf("pub(super) fn drops_given"));
  expect(given.slice(0, given.indexOf("\n}\n"))).toMatch(/state\.give\([\s\S]*\.drops\(ty\) != Drops::Nothing/);
});

// An iterator consumed by std's code is dropped where that code ends by one
// path, `consumed_call`, whichever lowers the call, `mir_iter_call` or a std
// combinator's (`consumed_by_std`, #1120): `C::from_iter(items)` had a
// lowering of its own beside `collect()`'s, which never dropped what it
// consumed, so `Vec::from_iter(it)` of an iterator with a destructor never
// dropped it, where `it.collect()` did (#1116). It's `items.into_iter()`
// then `collect()` now, as std's is, made so where `mir_iter_call` begins
// alone; a second reading of `from_iter`, or of what consumes and drops,
// fails here (ADR 0160, amended).
test("an iterator std's code consumes is dropped by mir_iter_call's consumed_call alone", () => {
  const lowering = [...files("crates/rust-js-lower/src").filter((f) => f !== "crates/rust-js-lower/src/lib.rs"), ...files("crates/rust-js-mir/src"), "src/pipeline.rs", ...files("src/pipeline")].filter((file) => file.endsWith(".rs") && !file.startsWith("crates/rust-js-lower/src/recognition"));
  const reading = (pattern: RegExp) => lowering.filter((file) => pattern.test(read(file)));
  expect(reading(/IterMethod::FromIter\b/)).toEqual(["crates/rust-js-mir/src/iter.rs"]);
  expect(read("crates/rust-js-mir/src/iter.rs").match(/IterMethod::FromIter\b/g)).toHaveLength(1);
  expect(reading(/\.consumed_call\(/)).toEqual(["crates/rust-js-mir/src/iter.rs"]);
  const iter = read("crates/rust-js-mir/src/iter.rs");
  const body = (name: string) => {
    const from = iter.slice(iter.indexOf(`fn ${name}(`));
    return from.slice(0, from.indexOf("\n    }\n"));
  };
  expect(body("mir_iter_call")).toContain("return self.collected_from_iter(");
  expect(body("mir_iter_call")).toContain("self.consumed_or_held(");
  expect(body("consumed_or_held")).toContain("self.consumed_call(");
  // And what a std combinator consumes, `extend`'s, `unzip`'s or
  // `partition`'s, before it's lowered (#1120).
  expect(body("consumed_by_std")).toContain("self.consumed_call(");
  const known = read("crates/rust-js-mir/src/dispatch/known_calls.rs");
  const value = known.slice(known.indexOf("fn mir_std_value("));
  expect(value.slice(0, value.indexOf("\n    }\n"))).toContain("self.consumed_by_std(");
});

// The runtime steps what may be a Rust iterator only by its stepping
// helpers, `$eachOf`, `$foldToArray`, `$extend` and `$extendText`, each of
// which closes it where its own step throws, a panic, as Rust drops it as it
// unwinds. JS's own, `for..of`, `Array.from` and a spread, close it only
// where their body throws, never its step: six helpers so stepped a
// `Map<vec::IntoIter<D>, F>` whose `F` panics, its `Vec`'s items never
// dropped (#1126, #1141). Each of these over a parameter named as an
// iterable is one of what's never a Rust iterator, said below by its file,
// how many and why; another fails here.
test("the runtime steps what may be a Rust iterator by its stepping helpers alone", () => {
  const steps = /(for \(const [^)]* of (items|it|iter|entries|iterable|replacement|units|source)\)|Array\.from\((items|it|iter|entries|iterable|replacement|units|source)[,)]|\.\.\.(items|it|iter|entries|iterable|replacement|units|source)\b)/;
  const never: Record<string, [number, string]> = {
    "crates/rust-js-program/src/runtime/btree.js": [4, "a B-tree made of a collect's array or another tree's entries"],
    "crates/rust-js-program/src/runtime/extract_if.js": [1, "a map's entries"],
    "crates/rust-js-program/src/runtime/fold_of.js": [1, "`$foldToArray`'s copy of an array"],
    "crates/rust-js-program/src/runtime/hashed_map.js": [4, "a map or a set made of a collect's array or another's entries or items, a map's entries given handles, or a clone's entries put under their source's bytes"],
    "crates/rust-js-program/src/runtime/hasher.js": [1, "a collection's items, hashed"],
    "crates/rust-js-program/src/runtime/index_map.js": [1, "a map's entries"],
    "crates/rust-js-program/src/runtime/key_map.js": [1, "a map made of a collect's array or another map's entries"],
    "crates/rust-js-program/src/runtime/key_set.js": [1, "a set made of a collect's array or another set's items"],
    "crates/rust-js-program/src/runtime/made.js": [1, "a slice, a deque, a map or a set being cloned"],
    "crates/rust-js-program/src/runtime/map_ops.js": [4, "a map's entries, or a collect's array, `$treeFrom`'s gated by `unwinding_collect`"],
    "crates/rust-js-program/src/runtime/mut_entries.js": [1, "a map's entries"],
    "crates/rust-js-program/src/runtime/nth.js": [1, "`$lastStep`'s JS iterator with no `last`, which every `$Steps` has"],
    "crates/rust-js-program/src/runtime/ord_map.js": [5, "a map made of a collect's array or another map's entries"],
    "crates/rust-js-program/src/runtime/ord_tree.js": [2, "a tree made of a collect's array or another tree's entries"],
    "crates/rust-js-program/src/runtime/rest.js": [1, "an iterator over an array, `steps_an_array`'s"],
    "crates/rust-js-program/src/runtime/steps.js": [1, "`$inTurnOf`'s drops, or a map's keys"],
  };
  const counts: Record<string, number> = {};
  for (const file of files("crates/rust-js-program/src/runtime").filter((file) => file.endsWith(".js"))) {
    const found = read(file).split("\n").filter((line) => !line.trimStart().startsWith("//") && steps.test(line)).length;
    if (found > 0) counts[file] = found;
  }
  const expected = Object.fromEntries(Object.entries(never).map(([file, [count]]) => [file, count]));
  expect(counts).toEqual(expected);
});

// A reversed zip is `$rev($zipEnds(a, b))`, `rev_items`' of what's made
// knowing its back, the one form; `$zipBack`, its second, is gone (ADR 0493,
// #740, the user's decision). A new one fails here.
test("a reversed zip has one form, never `$zipBack`", () => {
  const sources = [...files("crates/rust-js-lower/src"), ...files("crates/rust-js-mir/src"), "src/pipeline.rs", ...files("src/pipeline"), ...files("crates/rust-js-program/src/runtime")];
  const found = sources.filter((path) => read(path).includes("$zipBack"));
  expect([found, "a reversed zip is `$rev($zipEnds(a, b))` (ADR 0493)"]).toEqual([[], "a reversed zip is `$rev($zipEnds(a, b))` (ADR 0493)"]);
});

// When a collect asks its iterator's `size_hint`, and for what room, is
// `iterators/hint_asks.rs`'s one model of std's `SpecFromIter`, its
// `SpecFromIterNested` and `extend_desugared` and the reserves of a
// `String`'s and a hash table's `extend` (#820): six P0s in a month
// (#724 #728 #733 #794 #796 #797) were collects that asked it at the
// wrong time, or not at all, each decided where it was lowered. So the
// runtime's `$roomForHint`, `$reserveHint` and `$grownBy` (an `extend`'s,
// #849) are made there alone, and
// whether asking the hint may run the crate's own is asked there, by the
// analysis through `grows_by_callers_size`, and by `Filter::count`, which
// is no collect, through lengths.rs's `step_asks_own_hint` (#2210, #2112),
// alone.
test("when a collect asks its hint is hint_asks's alone", () => {
  // hint_asks.rs and its submodules, `hint_asks/unknown_extends.rs`'s
  // `extend` of what generic code is given among them.
  const owner = (file: string) => file === "crates/rust-js-lower/src/iterators/hint_asks.rs" || file.startsWith("crates/rust-js-lower/src/iterators/hint_asks/");
  const lowering = [...files("crates/rust-js-lower/src").filter((f) => f !== "crates/rust-js-lower/src/lib.rs"), ...files("crates/rust-js-mir/src"), "src/pipeline.rs", ...files("src/pipeline")].filter((file) => file.endsWith(".rs") && !owner(file));
  const code = (file: string) => read(file).split("\n").filter((line) => !/^\s*\/\//.test(line));
  const making = lowering.flatMap((file) =>
    code(file).filter((line) => /"\$(?:roomForHint|reserveHint|grownBy)"/.test(line)).map((line) => `${file}: ${line.trim()}`));
  expect(making).toEqual([]);
  const asking = lowering
    .filter((file) => !["crates/rust-js-lower/src/recognition/lengths.rs", "crates/rust-js-lower/src/shortcuts.rs"].includes(file))
    .filter((file) => code(file).some((line) => /(?<!fn )\bhint_reaches_own\(/.test(line)));
  expect(asking).toEqual([]);
});


// What a body's `impl Trait` hides is read once, where its MIR is taken
// (`mir::revealed`, #1142): each rule of the body's lowering asks the type
// its value is, so none of `crates/rust-js-mir/src/`, `iterators/` or
// `combinators/` reveals a type itself, and nothing else expands an opaque
// type. A guard that forgot to reveal answered of the opaque type, and its
// value was stepped and dropped as another's.
test("a body's impl Trait is revealed where its MIR is taken, alone", () => {
  expect(read("crates/rust-js-lower/src/analysis/prepared.rs")).toContain("(body, promoted) = revealed::revealed(tcx, (body, promoted));");
  const expanding = compilerRustFiles().filter((file) => file.endsWith(".rs") && read(file).includes("try_expand_impl_trait_type"));
  expect(expanding).toEqual(["crates/rust-js-lower/src/analysis/revealed.rs"]);
  const revealing = ["crates/rust-js-mir/src", "crates/rust-js-lower/src/iterators", "crates/rust-js-lower/src/combinators"]
    .flatMap(files)
    .filter((file) => file.endsWith(".rs") && /\b(self|this|cx)\.reveal\(/.test(read(file)));
  expect(revealing).toEqual([]);
});

// Whether an adapter is cloned apart (ADR 0525) is one answer,
// `iterators/stepping.rs`' `cloned_apart`, of what the lowering makes, of
// what the analysis marks made knowing its back, and of what a body makes
// in place or holds to clone: `body_queries.rs` had its own, of the same
// `Cloned` facts, twice (#1119, #1243).
test("whether an adapter is cloned apart is one function", () => {
  const defining = compilerRustFiles().filter((file) => file.endsWith(".rs") && /\bfn cloned_apart\b/.test(read(file)));
  expect(defining).toEqual(["crates/rust-js-lower/src/iterators/stepping.rs"]);
});

// Whether one of std's iterator methods keeps each item it's given, so
// leaves none with a destructor undropped, is one catalog,
// `recognition::keeps_each` (#1167): the combinators' and MIR's consumers'
// two lists answered it apart, and #1150 had to extend one of them.
test("whether a consumer keeps each item is one catalog", () => {
  const lowering = [...files("crates/rust-js-lower/src").filter((f) => f !== "crates/rust-js-lower/src/lib.rs"), ...files("crates/rust-js-mir/src"), "src/pipeline.rs", ...files("src/pipeline")].filter((file) => file.endsWith(".rs"));
  const defining = (name: string) => lowering.filter((file) => new RegExp(`fn ${name}\\b`).test(read(file)));
  expect(defining("keeps_each")).toEqual(["crates/rust-js-lower/src/recognition/methods.rs"]);
  expect(defining("leaves_none")).toEqual([]);
  expect(read("crates/rust-js-lower/src/iterators/mir_steps/closing.rs")).toContain("crate::recognition::keeps_each(name)");
});

// Which adapters a body makes in place is decided once, by its owner,
// `body_queries/in_place.rs`'s `decide`, which `lower/pipeline.rs` calls
// once the crate's `Recognition` exists and before the analysis reads the
// bodies (#1327): a body holds an undecided `InPlace` until then, which
// only `body_queries` makes or decides, and its readers ask it only
// `made_at`, which an undecided one refuses. Nothing else writes the set,
// and nothing reads it but by `made_at`.
// In two stages (#1681's step iii): the ground, then, once the analysis
// knows what's stepped from its back, what's held after all. Between them
// only `back_stepped` reads, the ground alone, of a `rev()`, which the
// second stage never takes out (`ground_made_at`); every other reader asks
// `made_at`, which refuses one read before both stages are decided.
// Each stage holds THIR calls, and a reader asks of MIR's call at a span,
// of each THIR call there (`each_call_at`), InPlace's own join, so no
// reader takes one of two a macro's repetition writes there of the other
// (#1791, ADR 1151).
// `back_stepped` reads whether a `rev()` is made in place of the ground
// alone (its held-`rev()` rule), before the held flip, which may take a
// `rev()` out after (#1681, ADR 1136, amended). The flip can't change what
// decided it: it flips no link given what the ground made, a call made in
// place or a `let` of one, so what `back_stepped` marked of an in-place
// `rev()`, a JS iterator, blocks that `rev()`'s flip. Both hold: the ground's
// read alone, the order (above), and the check in `held_links`.
test("back_stepped reads the ground's rev()s, and the flip is given nothing the ground made", () => {
  expect(read("crates/rust-js-lower/src/analysis/back_stepped/locals.rs")).toMatch(/callee == rev\n\s+&& !body\.facts\.in_place_adapters\.ground_made_at\(&body\.thir, \*fn_span\)/);
  const source = read("crates/rust-js-lower/src/body_queries/in_place.rs");
  const held = source.slice(source.indexOf("fn held_links"), source.indexOf("fn let_made"));
  expect(held).toContain("let ground = body.facts.in_place_adapters.ground();");
  expect(held).toContain("ExprKind::Call { ref args, .. } => args.iter().any(|&arg| in_place(arg, &held)),");
  expect(held).toMatch(/ExprKind::VarRef \{ id \} => made\.get\(&id\)\.is_some_and\(\|init\| ground\.contains\(init\)\)/);
});

// What a compiled adapter is given is std's struct (ADR 0871): the flip
// takes a held chain's links from its source outward, each while it's
// flipped, never one past a link made in place, `src.chain(v).map(f)`'s
// `map`, which stepped a `$lazyChain` as std's `Chain` struct (#1780).
test("the held flip flips from the source outward, while each link is flipped", () => {
  const source = read("crates/rust-js-lower/src/body_queries/in_place.rs");
  const held = source.slice(source.indexOf("fn held_links"), source.indexOf("pub(super) fn in_place_adapters"));
  expect(held).toContain("for link in chain.links.into_iter().rev() {");
  expect(held).toMatch(/Some\(link\) if !given && !callers => _ = held\.insert\(link\),\n\s+_ => break,/);
  expect(held.match(/flipped_link/g)?.length).toBe(2);
  expect(read("crates/rust-js-lower/src/iterators/transparent.rs")).toMatch(/each link's call, its outermost first/);
});

test("in-place adapters are decided in two stages, then read", () => {
  const lowering = [...files("crates/rust-js-lower/src"), ...files("crates/rust-js-mir/src"), "src/pipeline.rs", ...files("src/pipeline")].filter((file) => file.endsWith(".rs"));
  const outside = lowering.filter((file) => !file.startsWith("crates/rust-js-lower/src/body_queries"));
  const writes = outside.filter((file) => /in_place_adapters\s*=[^=]|InPlace::decided|in_place_adapters\.decide_held\(/.test(read(file)));
  expect(writes).toEqual([]);
  const reads = outside.flatMap((file) =>
    [...read(file).matchAll(/in_place_adapters\.(\w+)/g)].map((m) => `${file}: ${m[1]}`),
  );
  // And how a value of a held-compiled type is cloned, `makes_in_place`,
  // whether the body made one of that type in place (#1973): one answer
  // for how a link is made and how it's cloned.
  const ofOwner = new Set(["crates/rust-js-lower/src/analysis/back_stepped/locals.rs: ground_made_at", "crates/rust-js-lower/src/std_impls/clones.rs: makes_in_place"]);
  expect(reads.filter((read) => !read.endsWith(": made_at") && !ofOwner.has(read))).toEqual([]);
  expect(reads.filter((read) => read.endsWith(": ground_made_at"))).toEqual(["crates/rust-js-lower/src/analysis/back_stepped/locals.rs: ground_made_at"]);
  const source = read("crates/rust-js-lower/src/body_queries.rs");
  expect(source).toMatch(/pub struct InPlace \{\n    ground: Option<IndexSet<ExprId>>,\n    held: std::cell::OnceCell<IndexSet<ExprId>>,\n\}/);
  expect(source).toMatch(/\n    fn decided\(set: IndexSet<ExprId>\) -> Self \{/);
  // Its one join, each THIR call at MIR's span, and no reader of a node alone.
  const in_place = source.slice(source.indexOf("impl InPlace {"), source.search(/pub(?:\([^)]*\))? fn strip\(/));
  // Its API, whatever the crate cut widens it to (#2299): these three.
  expect([...in_place.matchAll(/pub(?:\([^)]*\))? fn (\w+)/g)].map((m) => m[1])).toEqual(["made_at", "ground_made_at", "makes_in_place"]);
  expect(in_place.match(/each_call_at\(thir, span, /g)?.length).toBe(2);
  const deciding = lowering.filter((file) => /decide_in_place\(/.test(read(file)));
  expect(deciding).toEqual(["src/pipeline.rs"]);
  // The second stage, decided once in the analysis: after `back_stepped`,
  // its one reader before it, and before `type_fact_params`, the first
  // reader of both stages.
  const held = lowering.filter((file) => /body_queries::decide_held\(/.test(read(file)));
  expect(held).toEqual(["crates/rust-js-lower/src/analysis.rs"]);
  const analysis = read("crates/rust-js-lower/src/analysis.rs");
  const at = (text: string) => analysis.indexOf(text);
  expect([at("back_stepped::of_crate(") < at("body_queries::decide_held("), at("body_queries::decide_held(") < at("type_fact_params(")]).toEqual([true, true]);
});

// Whether a value of std's adapter is std's struct, compiled for this build,
// is asked of its type, never its definition's name alone (#1401): the
// catalogue's queries of a value, a maker's, a type's or a call's, take the
// type, the made type or the call's arguments, so a type `{:?}` shows can be
// compiled where another of the same adapter isn't. A query by definition
// alone, `is_held_compiled_def`, was the name's answer for every value.
test("the compiled std catalogue is asked of a type, not a definition", () => {
  const source = read("crates/rust-js-lower/src/recognition/compiled_std.rs");
  const signature = (name: string) => source.match(new RegExp(`pub(?:\\([^)]*\\))? fn ${name}\\b[^{]*`))?.[0];
  for (const name of ["makes_held_compiled", "sized_in_place", "is_compiled_std_type", "is_held_compiled", "compiled_instance"]) {
    expect([name, /\bTy<|GenericArgsRef/.test(signature(name) ?? "")]).toEqual([name, true]);
  }
  const byDefinition = [...files("crates/rust-js-lower/src").filter((f) => f !== "crates/rust-js-lower/src/lib.rs"), ...files("crates/rust-js-mir/src"), "src/pipeline.rs", ...files("src/pipeline")].filter((file) => file.endsWith(".rs"))
    .filter((file) => /\bfn (is_held_compiled_def|makes_held_compiled_def|compiled_for_build_def)\b|\bis_held_compiled_def\(/.test(read(file)));
  expect(byDefinition).toEqual([]);
});

// The compiled std catalogue names each type by its `StdPath` row and
// finds it by that row's `DefId` (`std_path_id`), and asks a type which it
// is by `std_path_item`, never the text of its path, `std::iter::Map`
// stripped or compared (#1681 change 2, the DefId step). A row of methods
// is keyed by `Of`, std's struct by its `StdPath` or each primitive's impl
// of the row's trait (ADR 1206).
test("the compiled std catalogue names its types by StdPath, never a path's text", () => {
  for (const file of ["crates/rust-js-lower/src/recognition/compiled_std.rs", "crates/rust-js-lower/src/recognition/compiled_std/rows.rs"]) {
    const text = read(file);
    expect([file, /\bstd_path\(|"std::|"core::/.test(text)]).toEqual([file, false]);
  }
  const catalog = read("crates/rust-js-lower/src/recognition/compiled_std.rs");
  for (const name of ["COMPILED_STD", "COMPILED_STD_IO", "COMPILED_STD_NET", "HELD_COMPILED", "MEANING_ONLY"]) {
    expect([name, new RegExp(`const ${name}: &\\[\\(?(?:StdPath|Of)[,\\]]`).test(catalog)]).toEqual([name, true]);
  }
  expect(/enum Of \{\n    Struct\(StdPath\),\n    Primitives,\n\}/.test(catalog)).toBe(true);
});

// What JS's own iterator forms know, an `Iterator.prototype`'s and a range's
// `$lazyRange`, is `iterators::own_form`'s one answer (#1792, #1803): no
// other lowering file matches them by name, but the range's own module,
// which makes `$lazyRange` and reads it back as it rewrites its own form.
test("JS's own iterator forms are known by own_form alone", () => {
  const lowering = [...files("crates/rust-js-lower/src"), ...files("crates/rust-js-mir/src"), "src/pipeline.rs", ...files("src/pipeline")].filter((file) => file.endsWith(".rs"));
  const owners = ["crates/rust-js-lower/src/iterators.rs", "crates/rust-js-lower/src/std_types/range.rs"];
  const reads = lowering
    .filter((file) => !owners.includes(file))
    .flatMap((file) => {
      const source = read(file);
      return [
        ...[...source.matchAll(/helper_name\(\)\s*[!=]=\s*Some\((?:LAZY_RANGE|"\$lazyRange")\)/g)].map(() => `${file}: $lazyRange`),
        ...[...source.matchAll(/"values"\s*\|\s*"keys"\s*\|\s*"entries"/g)].map(() => `${file}: values/keys/entries`),
      ];
    });
  expect(reads).toEqual([]);
  expect(read("crates/rust-js-lower/src/iterators.rs")).toContain("pub(crate) fn own_form(e: &Expr) -> Option<OwnForm>");
});

// What a crate's function returns of its parameters, a reborrow or an
// iterator adapted in place, is pointers/returned.rs's alone, and which
// adapters keep their source in place is asked of in_place_source's owner,
// recognition/specialization.rs, no list of them elsewhere (#1934; ADR
// 0419, amended).
test("a function's return provenance is returned.rs's, and the in-place adapters specialization.rs's", () => {
  const rust = compilerRustFiles();
  const defining = (pattern: RegExp) => rust.filter((file) => pattern.test(read(file)));
  expect(defining(/\bfn (returned_from|returned_iterator|returns_its_parameter)\b/)).toEqual(["crates/rust-js-lower/src/pointers/returned.rs"]);
  expect(defining(/\bfn (adapts_in_place|impl_asks_of|in_place_traits)\b/)).toEqual(["crates/rust-js-lower/src/recognition/specialization.rs"]);
  // A walk of a function's result asks which adapters keep their source,
  // never names them.
  expect(read("crates/rust-js-lower/src/pointers/returned.rs")).toContain("adapts_in_place(");
  expect(read("crates/rust-js-lower/src/pointers/returned.rs")).not.toMatch(/"(map|filter|enumerate|zip|skip|take)"/);
});


// Each of `MEANING_ONLY`'s entries says why it's there, so the list can't
// hold an unproved hand form (ADR 1136, amended): on its way to being held
// as std's struct, of an issue; or kept, its hand form proved equal to
// std's, of an ADR and its proof, a test file or the copy matrix's rows,
// each of which exists.
test("each MEANING_ONLY entry is on its way, of an issue, or kept, of an ADR and the rows that prove it", async () => {
  const source = read("crates/rust-js-lower/src/recognition/compiled_std.rs");
  const list = source.match(/^const MEANING_ONLY: &\[StdPath\] = &\[\n([\s\S]*?)^\];/m);
  expect(list).not.toBeNull();
  const problems: string[] = [];
  let said: string[] = [];
  for (const line of list![1].split("\n")) {
    const comment = line.match(/^\s*\/\/ (.*)$/);
    if (comment) {
      said.push(comment[1]);
      continue;
    }
    const entry = line.match(/^\s*StdPath::(\w+),$/);
    if (!entry) continue;
    const why = said.join(" ");
    said = [];
    if (/^On its way: #\d+/.test(why)) continue;
    if (!/^Kept, proved equal:/.test(why)) {
      problems.push(`${entry[1]}: neither "On its way: #<issue>" nor "Kept, proved equal: .. (ADR <n>). Proof: .."`);
      continue;
    }
    const adr = why.match(/\(ADR (\d{4})\)/)?.[1];
    if (adr === undefined || !readdirSync(join(root, "docs", "decisions")).some((file) => file.startsWith(`${adr}-`))) {
      problems.push(`${entry[1]}: its ADR, ${adr ?? "none"}, isn't one`);
    }
    const proof = why.match(/Proof: (.*)$/)?.[1] ?? "";
    const paths = [...proof.matchAll(/(test\/[\w/.-]+\.(?:rs|ts))/g)].map((m) => m[1]);
    const rows = [...proof.matchAll(/ITERATOR_TABLE=(\w+) ITERATOR_MATRIX=(\/[^\s]+\/)/g)];
    if (paths.length + rows.length === 0) problems.push(`${entry[1]}: its proof names no test file or matrix rows`);
    for (const path of paths) {
      try {
        read(path);
      } catch {
        problems.push(`${entry[1]}: its proof's ${path} doesn't exist`);
      }
    }
    const { tableOf } = await import("./matrix-tables");
    for (const [, table, pattern] of rows) {
      const re = new RegExp(pattern.slice(1, -1));
      const found = tableOf(table as "copies").cases().filter((c: { id: string }) => re.test(c.id)).length;
      if (found === 0) problems.push(`${entry[1]}: its proof's ${table} rows ${pattern} are none`);
    }
  }
  expect(problems).toEqual([]);
});

// Whether a `copied()` is its source in JS is `is_its_source`'s alone
// (iterators/mir_steps/copies.rs, ADR 0480, revised for #2130): std's `Copied` doesn't
// pass `nth_back` on, so a copy of what writes it is a map of it. Each other
// reader of `recognition::copied_source` asks a question of the type that
// holds of a map too (its hint, its length, what steps it from its back),
// or asks `is_its_source` first; listed per file with its reason. A new
// reader fails, and so does a gone one until its row goes too.
test("whether a copy is its source is asked of is_its_source alone", () => {
  const allowed: Record<string, [number, string]> = {
    "crates/rust-js-lower/src/iterators/mir_steps/copies.rs": [5, "the owner: `is_its_source`, `copied_given` and the copy's maker"],
    "crates/rust-js-lower/src/iterators/knowing.rs": [2, "made knowing its back or its length: of its source where `is_its_source`, else `copy_made_knowing`"],
    "crates/rust-js-lower/src/iterators/hint_knowing.rs": [1, "made knowing its hint alone: of its source where `is_its_source`"],
    "crates/rust-js-lower/src/iterators/hint_asks.rs": [2, "its hint's parts, of its source where `is_its_source`; whether its hint is its parts', which a map's is too"],
    "crates/rust-js-lower/src/iterators/mir_steps/sources.rs": [2, "`counted_in_place`, a copy's count its source's, as std's `Copied::count` asks it; `steps_an_array`, where `is_its_source`"],
    "crates/rust-js-lower/src/std_impls/clones.rs": [1, "its clone, `copied_clone`, which asks `is_its_source`"],
    "crates/rust-js-lower/src/analysis/back_stepped.rs": [1, "what steps it from its back, a copy as a map: of a type"],
    "crates/rust-js-lower/src/analysis/back_stepped/locals.rs": [2, "what steps it from its back, a copy as a map: of a local"],
    "crates/rust-js-lower/src/analysis/hint_asked.rs": [1, "whether its hint is asked, as a map's"],
    "crates/rust-js-lower/src/iterators/in_place_forms.rs": [1, "the std type, `Copied`, whose form made knowing its hint alone is `$copied`, a question of its type"],
    "crates/rust-js-lower/src/recognition/lengths.rs": [2, "its hint's lower bound, its source's, as a map's"],
  };
  const found: Record<string, [number, string]> = {};
  for (const file of [...files("crates/rust-js-lower/src").filter((f) => f !== "crates/rust-js-lower/src/lib.rs"), ...files("crates/rust-js-mir/src"), "src/pipeline.rs", ...files("src/pipeline")].filter((file) => file.endsWith(".rs"))) {
    const count = (read(file).match(/copied_source\((self\.)?tcx/g) ?? []).length;
    if (count > 0) found[file] = [count, allowed[file]?.[1] ?? "unlisted"];
  }
  expect(found).toEqual(allowed);
});

// What a sized `Box` of an iterator is, for every question of the
// iterator, is `held_unboxed`'s alone (#2219, #2221): each owner that
// peeled a box by hand, or looked for a type parameter without peeling
// one, gave its own answer, so a `Box<I>` was closed field by field and
// dropped twice, never given its dictionary, asked no hint, never closed,
// or made knowing its back as an array. In the iterator owners a
// `boxed_ty()` is the owner's, or a question of the box itself: a
// `Box<dyn Iterator>`'s own row (ADR 0521), a boxed slice, or what std's
// `Box<I>` passes on of a compiled struct, which lends it "boxed" (#1995).
test("what a sized box of an iterator is, is held_unboxed's alone", () => {
  const owners = [
    "crates/rust-js-lower/src/recognition/held.rs",
    "crates/rust-js-lower/src/recognition/iterators.rs",
    "crates/rust-js-lower/src/recognition/lengths.rs",
    "crates/rust-js-lower/src/iterators/mir_steps/closing.rs",
    "crates/rust-js-lower/src/iterators/mir_steps/backs.rs",
    "crates/rust-js-lower/src/iterators/stepping.rs",
    "crates/rust-js-lower/src/iterators/knowing.rs",
  ];
  const peels = owners.flatMap((file) =>
    read(file)
      .split("\n")
      .map((line) => line.replace(/\/\/.*$/, "").trim())
      .filter((line) => line.includes("boxed_ty()"))
      .map((line) => `${file}: ${line}`),
  );
  expect(peels).toEqual([
    // `iterator_held`: which impl a type's steps reach, through any box.
    "crates/rust-js-lower/src/recognition/held.rs: while let Some(inner) = ty.boxed_ty() {",
    // `held_unboxed` itself.
    "crates/rust-js-lower/src/recognition/held.rs: .boxed_ty()",
    // `behind_a_box`: what std's `Box<I>` passes on (`Wrapper::passes`).
    "crates/rust-js-lower/src/recognition/held.rs: ty.peel_refs().boxed_ty().is_some()",
    // A `Box<dyn Iterator>`, closed by the drop it carries (ADR 0521).
    "crates/rust-js-lower/src/recognition/iterators.rs: let boxed_dyn = source.boxed_ty().is_some_and(|inner| self.is_dyn_iter(inner));",
    // `dyn_iter_trait`: a `dyn`'s, boxed or not.
    "crates/rust-js-lower/src/recognition/iterators.rs: let ty = ty.boxed_ty().unwrap_or(ty);",
    // The one box `held_unboxed` keeps, a `Box<dyn Iterator>`'s hint.
    "crates/rust-js-lower/src/recognition/lengths.rs: let held = ty.boxed_ty().unwrap_or(ty);",
    // A boxed slice, a sequence, not an iterator.
    "crates/rust-js-lower/src/iterators/stepping.rs: && (self.is_vec_like(collection) || collection.boxed_ty().is_some_and(|t| t.is_slice()))",
    "crates/rust-js-lower/src/iterators/stepping.rs: let ty = ty.boxed_ty().unwrap_or(ty);",
    // Which parameter a callee takes an `IntoIterator`'s `IntoIter` by.
    "crates/rust-js-lower/src/iterators/stepping.rs: _ => declared.boxed_ty().unwrap_or(*declared) == input,",
    // A box of std's compiled struct, lent "boxed" (#1995).
    "crates/rust-js-lower/src/iterators/knowing.rs: if let Some(inner) = ty.boxed_ty()",
  ]);
  expect(read("crates/rust-js-lower/src/recognition/held.rs")).toContain("pub(crate) fn held_unboxed");
});

// Whether a projection's iterator is its type's own value, stepped by its
// trait's dictionary, or a JS iterator, is one answer: `projection_as_it_is`
// in generic code, of std's `IntoIterator`'s `IntoIter` and any other
// trait's associated iterator alike, and `assoc_iterator_as_it_is`, which
// `held_by_generic_code` asks of an impl's (ADR 0061, amended, #2330). A
// second decider, a projection taken for a JS iterator by its kind alone,
// `ty::Alias(..) => true`, gave one value two representations: a range given
// back to concrete code read as `undefined..undefined`.
test("a projection's iterator is told its type's own value by projection_as_it_is alone", () => {
  const defines = /\bfn (projection_as_it_is|assoc_iterator_as_it_is)\b/;
  expect(compilerRustFiles().filter((file) => defines.test(read(file)))).toEqual(["crates/rust-js-lower/src/iterators.rs"]);
  const byKind = compilerRustFiles()
    .filter((file) => file.startsWith("crates/rust-js-lower/src/iterators") || file.startsWith("crates/rust-js-lower/src/traits"))
    .flatMap((file) => read(file).split("\n").filter((line) => /^\s*ty::Alias\(\.\.\) => true/.test(line)).map((line) => `${file}: ${line.trim()}`));
  expect(byKind).toEqual([]);
});

// A consumer of std's adapter takes its own `count` or `last` where
// `owns_its_step` says, the one answer: whether std's step of it may be
// observably its own, as its committed runs ask (`observably_owns`), and
// what it asks reaches the crate's own or a hint that may run its code
// (#2112, Change A step 3a). So no lowering asks `reaches_its_own` of
// `count` or `last` itself, which would take a form's own step where
// std's is its default; `nth` and `nth_back` aren't asked so until #2213.
// Its meaning, each consumer over each in-place form, is the consumer
// net's (test/consumer-net.ts, `inPlaceForms`).
test("a consumer asks owns_its_step of count and last, never reaches_its_own", () => {
  const lowering = [...files("crates/rust-js-lower/src"), ...files("crates/rust-js-mir/src")].filter(
    (file) => file.endsWith(".rs") && file !== "crates/rust-js-lower/src/recognition/own_steps.rs",
  );
  const code = (file: string) => read(file).split("\n").filter((line) => !/^\s*\/\//.test(line)).join("\n");
  // Of `count` or `last` named in the call, or in a list of steps handed
  // to it a few lines before.
  const asks = /reaches_its_own\([^()]*OwnStep::(?:Count|Last)\b|\[[^\]]*OwnStep::(?:Count|Last)\b[^\]]*\][^;]{0,160}?reaches_its_own\(/;
  const wrong = lowering
    .filter((file) => asks.test(code(file)))
    .map((file) => `${file}: asks \`reaches_its_own\` of \`count\` or \`last\`; ask \`owns_its_step\`, the one answer of whether a consumer takes the form's own step`);
  expect(wrong).toEqual([]);
});

// A fallible collect's helpers, `$collectOptions` and `$collectResults`,
// have one maker, `fallible_collect_call` (iterators/stepping.rs), which
// says what drops the `Err` it keeps and whether a `Some` is boxed of the
// item itself: a `sum` into a `Result` that made `$collectResults` its own
// way gave no `dropErr`, and the `Err` was never dropped as its source's
// drop panicked (#2574, #2572's first piece of one owner).
test("a fallible collect's helpers have one maker", () => {
  const makers = compilerRustFiles().flatMap((file) =>
    [...read(file).matchAll(/"\$collect(?:Options|Results)"/g)].map(() => file),
  );
  expect([...new Set(makers)]).toEqual(["crates/rust-js-lower/src/iterators/stepping.rs"]);
  const stepping = read("crates/rust-js-lower/src/iterators/stepping.rs");
  const maker = stepping.slice(stepping.indexOf("pub(crate) fn fallible_collect_call("));
  const body = maker.slice(0, maker.indexOf("\n    }\n"));
  expect(body).toContain('Expr::helper("$collectOptions")');
  expect(body).toContain('Expr::helper("$collectResults")');
  expect(stepping.split('"$collect').length - 1).toBe(2);
});

// Who drops what std's code takes by value is one answer, `consumed_drop`
// (iterators/mir_steps/consumed.rs), its rules its own cases: each side
// asked its own predicates, `consumer_closes` and the rest, and combined
// them its own way, so one closed what the other dropped after, or neither
// did (#2561, #2580). No predicate of it is written elsewhere, a function
// that says whether a consumer closes; MIR's `dropped_after` reads its
// answer; and its readers are the closing sites' one, `consumer_closing`,
// an `rfold`'s and `dropped_after` (#2572, ADR 1205).
test("who drops a consumed iterator is consumed_drop's answer alone", () => {
  const compiler = compilerRustFiles();
  const deciders = compiler.filter((file) => /\bfn \w+_closes\s*[<(]/.test(read(file)));
  expect(deciders).toEqual([]);
  const iter = read("crates/rust-js-mir/src/iter.rs");
  const dropped = iter.slice(iter.indexOf("    pub(crate) fn dropped_after("), iter.indexOf("\n    }\n", iter.indexOf("    pub(crate) fn dropped_after(")));
  expect(dropped).toContain("self.consumed_drop(");
  const readers = compiler.filter((file) => /\.consumed_drop\(/.test(read(file)));
  expect(readers).toEqual([
    "crates/rust-js-lower/src/iterators/mir_steps/backs.rs",
    "crates/rust-js-lower/src/iterators/mir_steps/consumed.rs",
    "crates/rust-js-mir/src/iter.rs",
  ]);
});

// A source is closed one of two ways, each a named helper of steps.js:
// `$closeHolding(it, ...held)`, a collector's, which drops what it holds
// where the close panics, as std's callee's frame drops it, and
// `$closeBare(it)`, of what holds nothing of its own beside it. No
// `.return()` of a source is written elsewhere in the runtime, so a
// collector can't close what it was given and leak what it made (#2564,
// #2572).
test("the runtime closes a source by $closeHolding or $closeBare alone", () => {
  const runtime = files("crates/rust-js-program/src/runtime").filter((file) => file.endsWith(".js") && !file.endsWith("std_sites.js"));
  const helpers = ["  return $unwinds(() => it.return?.(), ...held);", "  return it?.return?.();"];
  const bare = runtime.flatMap((file) =>
    read(file)
      .split("\n")
      .filter((line) => !line.trimStart().startsWith("//") && /\.return(\?\.)?\(/.test(line) && !helpers.includes(line))
      .map((line) => `${file}: ${line.trim()}`),
  );
  expect(bare).toEqual([]);
  const steps = read("crates/rust-js-program/src/runtime/steps.js");
  expect(steps).toContain("function $closeHolding(it, ...held) {");
  expect(steps).toContain("function $closeBare(it) {");
});
