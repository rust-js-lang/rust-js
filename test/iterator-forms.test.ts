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
import { readFileSync } from "node:fs";
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
];
const which = process.env.ITERATOR_FORMS;
if (which !== undefined && which !== "corpus") throw new Error(`ITERATOR_FORMS=${which}: \`corpus\`, or unset for a few cases`);
const chosen = which === "corpus" ? caseShard(cases) : few;

/** The cases whose std-struct build doesn't run as native Rust yet, each
 * with why: what's left to do, which only shrinks. One that runs as Rust
 * does is taken off. */
const differs: Record<string, string> = {
  "asked_length_overflow.rs": "runs on: prints without end",
  "back_stepped_adapter_sources.rs": "refused: the DoubleEndedIterator of a Chain<slice::Iter, slice::Iter>",
  "back_steps.rs": "throws: a recognizer's JS iterator stepped from its back by std's struct",
  "boxed_array_iter.rs": "refused: a std::boxed::BoxedArrayIntoIter<u8, 2> as a DoubleEndedIterator, which rust-js can't step from its back",
  "btree_given_in_order.rs": "rust-js crashes lowering it",
  "copied_structs_apart.rs": "refused: the DoubleEndedIterator of a Copied<slice::Iter>",
  "escape_iterators.rs": "throws: std's struct steps a value that isn't a JS iterator",
  "evidence_paths.rs": "refused: stepping I, which rust-js found nothing here steps",
  "flat_map_drops.rs": "refused: a std type holding a value with a destructor, Flatten<std::vec::IntoIter<std::option::Option<Loud>>>,",
  "flat_map_inner_iterables.rs": "rust-js crashes lowering it",
  "flatten_from_its_back.rs": "throws: a recognizer's JS iterator stepped from its back by std's struct",
  "flatten_of_iterators_from_back.rs": "throws: a recognizer's JS iterator stepped from its back by std's struct",
  "fold_back_drops_what_it_takes.rs": "refused: the DoubleEndedIterator of a Skip<Count>",
  "held_enumerate.rs": "refused: the DoubleEndedIterator of a R",
  "lazy_iterator_clone.rs": "throws: std's struct clones a function the recognizers made",
  "own_back_through_adapters.rs": "refused: the DoubleEndedIterator of a Take<Count>",
  "own_iterator_dropped_from_back.rs": "prints otherwise than native Rust",
  "own_iterator_dropped_through_adapters.rs": "refused: the DoubleEndedIterator of a Skip<Fuse<Count>>",
  "rev_in_place_through_adapters.rs": "refused: the DoubleEndedIterator of a Skip<Rev<Count>>",
  "rev_of_lazy_chains.rs": "throws: a recognizer's JS iterator stepped from its back by std's struct",
  "rev_own_back_methods.rs": "prints otherwise than native Rust",
  "skip_try_fold.rs": "prints otherwise than native Rust",
  "std_map.rs": "throws: std's struct clones a function the recognizers made",
  "zip_from_back.rs": "refused: a Zip<std::slice::IterMut<'_, i32>, std::slice::IterMut<'_, i32>> as a DoubleEndedIterator, which rust-js can't step from its back",
};

test("the cases a run of a few names are the corpus's, and each listed difference is one", () => {
  for (const name of [...few, ...Object.keys(differs)]) expect(cases).toContain(name);
});

for (const name of chosen.filter((name) => cases.includes(name))) {
  const file = join(corpus, name);
  const want = directives(readFileSync(file, "utf8"));
  // What doesn't run as Rust does yet, or doesn't run, has no form to
  // check; a malformed directive is the corpus's to report.
  if (typeof want === "string" || want.kind === "compile-fail" || want.kind === "ignore-rust-js") continue;
  test(`${basename(name, ".rs")} with std's iterator structs`, () => {
    const dir = fixture(`forms-${basename(name, ".rs")}`);
    const native = runNative(file, dir, want.edition, want.native);
    if (typeof native === "string") throw new Error(native);
    const compiled = compileJs(file, dir, want.edition, false, {}, ["--std-iterators"]);
    let problem = "";
    if (!("js" in compiled)) {
      problem = `rust-js failed: ${compiled.error.split("\n").slice(0, 6).join("\n")}`;
    } else {
      const js = readFileSync(compiled.js, "utf8");
      const key = runKey(["std iterators", js, native.bytes.stdout, native.bytes.stderr, show(native.outcome), shared()]);
      if (!verified(key)) {
        const [runtime, cmd] = runtimes[0];
        const ran = runJs(cmd, compiled.js, dir, `${runtime}-std-iterators`);
        if (agree(ran, native)) record(key, `${name} with std's iterator structs`);
        else problem = `${show(ran.outcome)}, printing\n${ran.stdout.slice(0, 400)}\nwhere native Rust ended ${show(native.outcome)}, printing\n${native.stdout.slice(0, 400)}`;
      }
    }
    if (name in differs) expect(problem, `${name} runs as Rust does now: take it off \`differs\``).not.toBe("");
    else expect(problem).toBe("");
  }, 300_000);
}
