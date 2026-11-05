// The iterator rules of the architecture, split from architecture.test.ts at
// its cap (a test file is at most 1500 lines): who gives an iterator's steps,
// which own step an adapter reaches, and who answers whether stepping it may
// panic (ADR 1089).
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { iteratorPaths, touchesIterators } from "../scripts/iteratorSlice";
import { root } from "./support";

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
  const giving = files("src/runtime")
    .filter((file) => file.endsWith(".js") && file !== "src/runtime/forward.js")
    .filter((file) => read(file).split("\n").some((line) => !line.trimStart().startsWith("//") && given.test(line)));
  expect(giving).toEqual([]);
});

// Which of the crate's own steps std's adapters reach is one catalog's,
// `OwnStep::asked_of`, and one query's, `reaches_its_own` (ADR 1089): a
// predicate per step, each with its own list of adapters, treated a type
// parameter and a `&mut` each its own way (#496, #505, #521).
test("which own step an adapter reaches is the OwnStep catalog's alone", () => {
  const lowering = ["src/lower.rs", ...files("src/lower")].map(read).join("\n");
  expect(lowering).not.toMatch(/fn (counts|lasts|folds|nths|rfolds|try_rfolds|nth_backs|may_fold)_its_own\b/);
  expect(read("src/lower/recognition/own_steps.rs")).toContain("fn asked_of(self, adapter: &str)");
});

// What an adapter's fold or `nth`, from either end, asks of what it holds is
// that catalog's too, wherever another question follows it: slice_steps.rs
// kept its own list of adapters whose `rfold` asks an `nth_back`, without the
// `peekable()` the catalog had, so generic code's `take(3).peekable().rev()`
// was collected by `next`, dropping 4 before 5 where std drops 5 before 4,
// and the list of those whose `nth` asks one, the catalog's own row (#697).
// A function of the lowering named for a fold or an `nth` names no adapter of
// std's but in own_steps.rs, which asks it of `asked_sources`.
test("which source an adapter's fold or nth asks is the OwnStep catalog's alone", () => {
  const adapter = /"(?:std::iter::)?(Map|Filter|FilterMap|Take|Skip|Fuse|Peekable|Enumerate|StepBy|Chain|Rev|Inspect|Copied|Cloned|Flatten|FlatMap|SkipWhile|TakeWhile|Scan|MapWhile|Zip)"/;
  const listed: string[] = [];
  for (const file of ["src/lower.rs", ...files("src/lower")].filter((file) => file.endsWith(".rs"))) {
    if (file === "src/lower/recognition/own_steps.rs") continue;
    for (const item of read(file).split(/\n(?=\s*(?:pub(?:\([^)]*\))? )?fn \w+)/)) {
      const name = item.match(/fn (\w+)/)?.[1];
      if (name !== undefined && /fold|nth/.test(name) && adapter.test(item)) listed.push(`${file}: fn ${name}`);
    }
  }
  expect(listed).toEqual([]);
  expect(read("src/lower/recognition/slice_steps.rs")).toContain("self.asked_sources(ty, OwnStep::Nth)");
});

// Whether stepping an iterator may panic, which decides what a fold's
// `unwind` and a collect's `$collectDropping` drop, is `steps_may_panic`'s
// alone, of the iterator's type (ADR 1089): its JS's arrows, `chain_may_throw`,
// and its `$closing`s, `closes_a_source`, answered it too, and disagreed
// with the type where a compiled `fuse()` closed nothing of a `Vec`'s rest
// (#653). Neither gate is given the JS it gates.
test("whether an iterator's steps may panic is steps_may_panic's alone", () => {
  const collect = read("src/lower/mir/iter/collect.rs");
  for (const gate of ["unwinding_collect", "stranded_drop"]) {
    const signature = collect.slice(collect.indexOf(`fn ${gate}(`), collect.indexOf(") ->", collect.indexOf(`fn ${gate}(`)));
    expect(signature, gate).not.toContain("Expr");
  }
  const lowering = ["src/lower.rs", ...files("src/lower")].map(read).join("\n");
  expect(lowering).not.toMatch(/fn (chain_may_throw|closes_a_source|callbacks_in)\b/);
});

// An adapter that holds a function with a destructor is `$holding(adapter,
// drop, moved)` alone, which `closing_function` makes, so its function drops
// where std's fold drops it (#710): `$closing(adapter, (it) => { it.return();
// drop f })`, the form it replaced, dropped it after what a `take` left
// wherever a fold by value moved it. No lowering writes that arrow, and no
// JS the tests pin nor the guide holds it.
test("an adapter that holds a function is $holding alone", () => {
  const lowering = ["src/lower.rs", ...files("src/lower")].filter((file) => file.endsWith(".rs"));
  const making = lowering.filter((file) => read(file).includes('"$holding"'));
  expect(making).toEqual(["src/lower/iterators.rs", "src/lower/mir/iter.rs"]);
  expect(read("src/lower/mir/iter.rs")).toContain('Ok(Expr::call_helper("$holding", args))');
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
  const lacking = files("src/runtime")
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
  const lacking = files("src/runtime")
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
  const lowering = ["src/lower.rs", ...files("src/lower")].filter((file) => file.endsWith(".rs"));
  const writing = lowering.filter((file) => read(file).includes('"$consume"'));
  for (const file of writing) expect(read(file), file).toContain("consumed_ends(");
  expect(writing.sort()).toEqual(["src/lower/mir/iter/backs.rs", "src/lower/mir/iter/collect.rs"]);
  const asking = lowering.filter((file) => read(file).includes("consumed_function("));
  expect(asking.sort()).toEqual(["src/lower/mir/dispatch.rs", "src/lower/mir/iter/collect.rs"]);
  expect(read("src/runtime/fold_of.js")).toContain("function $consume(it, step, init, f, unwind, ends)");
});

// What the iterators' code is, which runs the iterator matrix's slice in a
// push's check and in check:fast (#731), is `iteratorPaths` alone: each
// module docs/architecture.md says owns a rule of iterators or their steps
// is one of its paths, and each of its paths is a file there is.
test("the iterator matrix's slice runs of each module that owns a rule of iterators", () => {
  const owners = read("docs/architecture.md")
    .split("\n")
    .filter((line) => line.startsWith("| ") && /iterat|step/i.test(line.split("|")[1] ?? ""))
    .flatMap((line) => [...(line.split("|")[2] ?? "").matchAll(/`([\w/]+(?:\.rs|\.js|\/))`/g)].map((m) => m[1]))
    .map((module) => (module.startsWith("src/") ? module : `src/lower/${module}`))
    .map((module) => (module.endsWith("/") ? `${module}x.rs` : module));
  expect(owners).toContain("src/lower/recognition/own_steps.rs");
  expect(owners.filter((module) => !touchesIterators([module]))).toEqual([]);
  const tracked = Bun.spawnSync(["git", "ls-files", "--cached", "--others", "--exclude-standard"], { cwd: root }).stdout.toString().split("\n");
  expect(iteratorPaths.filter((path) => !tracked.some((file) => new Bun.Glob(path).match(file)))).toEqual([]);
  expect(touchesIterators(["src/lower/jsx.rs", "docs/guide/README.md"])).toBe(false);
});

// An expression rebuilt of its own kind keeps what its maker marked it,
// `facts: its_facts(e.facts)` (lower/iterators/value.rs): the iterator
// value's marks are the one way a reader past its maker knows what it is
// (ADR 1092, amended). `iterated`, `from_iterator` and tidy's `unarrayed`
// rebuilt a range stepped from its end with `Facts::NONE`, and a `for` over
// `(a..b).rev()` stopped counting down (#730).
test("an expression rebuilt of its own kind keeps its value's marks", () => {
  const rebuilt = /Expr \{\s*facts:\s*(?:js::)?Facts::NONE,\s*kind,/;
  const dropping = ["src/lower.rs", ...files("src/lower")]
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
  const declared = files("src/runtime")
    .filter((file) => file.endsWith(".js"))
    .flatMap((file) => [...read(file).matchAll(chains)].map((m) => m[1]));
  expect(declared).toEqual(["$lazyChain"]);
  const lowering = ["src/lower.rs", ...files("src/lower")].filter((file) => file.endsWith(".rs"));
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
  const by = files("src/runtime")
    .filter((file) => file.endsWith(".js"))
    .flatMap((file) => back(read(file).split("\n")).map((line) => `${file}:${line}`));
  expect(by).toEqual([]);
  expect(read("src/runtime/steps.js")).toContain("function $inTurn(...runs)");
});

// An iterator value's facts are written once, by lower/iterators/value.rs's
// `emit()`, and read only through value.rs, which reads the `Known` facts
// `emit()` marks its JS with (ADR 1092, amended, #730 phase 3). A reader that
// took a helper's name, a call's shape or a span for a fact met a value of
// another form and answered wrongly: `.of` (#721), a closure's result judged
// by another call's span (#716, #726), a length off an assumed shape (#745).
// A new fact goes to value.rs: `emit()` marks it, and value.rs gives the
// reader its accessor.
const valueOwner = "src/lower/iterators/value.rs";
const owned = (what: string) =>
  `${what} outside value.rs: ${valueOwner} owns an iterator value's form; add a fact \`emit()\` marks, and an accessor there`;
const loweringSources = () => ["src/lower.rs", ...files("src/lower")].filter((file) => file.endsWith(".rs"));
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

test("$iterator, $given, $knowing, $closing, $lent, $rev, $stepBy and $repeatingN are written by value.rs alone", () => {
  const written = /(call_helper|Expr::helper)\("\$(iterator|given|knowing|closing|lent|rev|stepBy|repeatingN)"/;
  expect([owned("a write of an iterator value's helper"), outsideValue(written)]).toEqual([owned("a write of an iterator value's helper"), []]);
});

test("no iterator fact is read off a member's name, `.of`, `.at`, `.end`, `.iter` or `.items`, outside value.rs", () => {
  const member = /Member\([^)]*\)\s*if\s+\w+\s*==\s*"(of|at|end|iter|items)"|Member\(_,\s*"(of|at|end|iter|items)"/;
  expect([owned("a member read as an iterator's fact"), outsideValue(member)]).toEqual([owned("a member read as an iterator's fact"), []]);
});

// Whether a call steps an adapter made in place is body_queries' answer, read
// by mir/iter.rs's `steps_in_place` alone, the maker of each value's in-place
// mark (step 6). It's still keyed by the call's span, which #730's step 6b
// removes, each in-place adapter's maker marking what it makes.
test("in_place_adapter_steps is read by steps_in_place alone", () => {
  const allowed: Record<string, [number, string]> = {
    "src/lower/body_queries.rs": [4, "its owner, which makes it of the THIR"],
    "src/lower/analysis.rs": [1, "the `--std-iterators` build's switch, which makes no adapter in place"],
    "src/lower/mir/iter.rs": [1, "`steps_in_place`, its one reader, until #730's step 6b"],
  };
  const found: Record<string, number> = {};
  for (const file of loweringSources()) {
    const count = linesOf(file).filter((line) => line.includes("in_place_adapter_steps")).length;
    if (count > 0) found[file] = count;
  }
  const expected = Object.fromEntries(Object.entries(allowed).map(([file, [count]]) => [file, count]));
  expect(found).toEqual(expected);
});

// No iterator helper is read by its name outside value.rs: each reader asks
// value.rs's accessor of the fact `emit()` marked (steps 1 to 11). A new read
// fails here, with the owner to add the accessor to.
test("no iterator helper is read by its name outside value.rs", () => {
  const helpers = "closing|iterator|given|lent|stepBy|rev|repeatingN|knowing";
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
