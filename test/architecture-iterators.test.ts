// The iterator rules of the architecture, split from architecture.test.ts at
// its cap (a test file is at most 1500 lines): who gives an iterator's steps,
// which own step an adapter reaches, and who answers whether stepping it may
// panic (ADR 1089).
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
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
