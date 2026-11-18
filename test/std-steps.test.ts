// std's steps, the compiler's one catalog of them (#2112):
// src/lower/recognition/std_steps.txt, which `recognition::std_steps`
// reads, and test/iterator-steps.test.ts holds the runtime's adapters to.
// Made here from std alone, two ways: which of `Iterator`'s,
// `DoubleEndedIterator`'s and `ExactSizeIterator`'s methods each adapter
// and wrapper writes itself, read from its impls in this toolchain's
// rust-src; and what each of those steps asks of what it holds, run
// natively, test/std-steps.rs's probe, each of its sources printing each
// step it's asked with its argument. A toolchain whose std writes or asks
// another fails here, not as a wrong answer; `BLESS=1` writes the catalog
// again, whose diff is what std changed.

import { expect, test } from "bun:test";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { underLoad } from "../scripts/timing";
import { contentDirectory, nativeBinary, root } from "./support";

const catalogFile = join(root, "src/lower/recognition/std_steps.txt");
const probeFile = join(root, "test/std-steps.rs");

const sysroot = Bun.spawnSync(["rustc", "--print", "sysroot"], { cwd: root }).stdout.toString().trim();
const library = join(sysroot, "lib/rustlib/src/rust/library");
const read = (file: string) => readFileSync(join(library, file), "utf8");

/** The steps the probe asks, each a method of one of the three traits. */
export const steps = ["next", "size_hint", "nth", "advance_by", "fold", "try_fold", "last", "count", "next_back", "nth_back", "advance_back_by", "rfold", "try_rfold", "len", "is_empty"];

/** Where std writes each adapter's and wrapper's impls: its file, and how
 * each impl's header names its type, a pattern, `Map<`. */
const writers: [name: string, file: string, type: string][] = [
  ...(
    [
      ["Map", "map"],
      ["Filter", "filter"],
      ["FilterMap", "filter_map"],
      ["Enumerate", "enumerate"],
      ["Skip", "skip"],
      ["Take", "take"],
      ["StepBy", "step_by"],
      ["Chain", "chain"],
      ["Zip", "zip"],
      ["Rev", "rev"],
      ["Fuse", "fuse"],
      ["Peekable", "peekable"],
      ["Flatten", "flatten"],
      ["FlatMap", "flatten"],
      ["Inspect", "inspect"],
      ["Copied", "copied"],
      ["Cloned", "cloned"],
      ["TakeWhile", "take_while"],
      ["SkipWhile", "skip_while"],
      ["MapWhile", "map_while"],
      ["Scan", "scan"],
      ["Cycle", "cycle"],
    ] as const
  ).map(([name, file]): [string, string, string] => [name, `core/src/iter/adapters/${file}.rs`, `${name}<`]),
  ["&mut I", "core/src/iter/traits/*.rs", "&(?:'\\w+ )?mut I \\{"],
  ["&mut dyn I", "core/src/iter/traits/*.rs", "&(?:'\\w+ )?mut I \\{"],
  ["Box<I>", "alloc/src/boxed/iter.rs", "Box<I, A> \\{"],
  ["Box<dyn I>", "alloc/src/boxed/iter.rs", "Box<I, A> \\{"],
];

/** The methods of the three traits `name`'s impls write, of `steps`, in
 * their order: each impl's `fn`s and what `impl_fold_via_try_fold!` makes. */
function written(file: string, type: string): string[] {
  const files = file.endsWith("*.rs") ? ["iterator.rs", "double_ended.rs", "exact_size.rs"].map((f) => file.replace("*.rs", f)) : [file];
  const found = new Set<string>();
  let impls = 0;
  for (const text of files.map(read)) {
    const header = new RegExp(`^impl(?:<[^\\n]*?>)?\\s+(?:Iterator|DoubleEndedIterator|ExactSizeIterator) for ${type}`, "gm");
    for (const m of text.matchAll(header)) {
      impls++;
      const body = text.slice(m.index, text.indexOf("\n}\n", m.index));
      for (const f of body.matchAll(/^ {4}(?:default )?(?:unsafe )?fn (\w+)/gm)) found.add(f[1]!);
      for (const f of body.matchAll(/impl_fold_via_try_fold! \{ (\w+) -> \w+ \}/g)) found.add(f[1]!);
    }
  }
  if (impls === 0) throw new Error(`std has no impl for \`${type}\` in ${file}: read where it's written again`);
  return steps.filter((step) => found.has(step));
}

/** What the probe prints, run natively. */
function probed(): string {
  const source = readFileSync(probeFile, "utf8");
  const built = nativeBinary(source, contentDirectory(source), ["--edition=2024", "-Awarnings"]);
  if ("error" in built) throw new Error(built.error);
  const ran = Bun.spawnSync([built.binary], { timeout: 30_000, maxBuffer: 1 << 20 });
  if (ran.exitCode !== 0) throw new Error(`the probe failed: ${ran.stderr.toString()}`);
  return ran.stdout.toString();
}

/** The catalog: a heading; then of each adapter and wrapper, the steps
 * std's impls write, and each run's line of what each step asked. */
function catalog(): string {
  const runs = probed().trimEnd().split("\n");
  const lines = [
    "# std's steps (#2112), made by test/std-steps.test.ts from this toolchain's std",
    "# (`BLESS=1 bun test test/std-steps.test.ts` writes it; never by hand).",
    "# `<name> writes:` the methods of Iterator, DoubleEndedIterator and",
    "# ExactSizeIterator std's impls of it write, read from rust-src; each",
    "# `<name>#<run>` line, what each step asked of what it holds, run natively",
    "# (test/std-steps.rs): `src` its source, `other` a chain's or a zip's",
    "# second, `inner` a flatten's inner iterators.",
  ];
  for (const [name, file, type] of writers) {
    lines.push(`${name} writes: ${written(file, type).join(" ")}`);
    const own = runs.filter((line) => line.startsWith(`${name}#`));
    if (own.length === 0) throw new Error(`the probe ran no \`${name}\``);
    lines.push(...own.map((line) => line.trimEnd()));
  }
  const named = new Set(writers.map(([name]) => name));
  expect(runs.filter((line) => !named.has(line.slice(0, line.indexOf("#"))))).toEqual([]);
  return `${lines.join("\n")}\n`;
}

test("the catalog of std's steps is what std writes and asks", () => {
  const made = catalog();
  if (process.env.BLESS) writeFileSync(catalogFile, made);
  expect(existsSync(catalogFile)).toBe(true);
  expect(readFileSync(catalogFile, "utf8")).toBe(made);
}, underLoad(120_000));
