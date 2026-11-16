// What std's `&mut I` and `Box<I>` pass on to the iterator they hold,
// `$lent`'s lists (src/runtime/lent.js's `$passedOn`) and the compiler's
// `BOX_PASSES_ON` (src/lower/recognition/iterators.rs), read from std's own
// source in this toolchain's rust-src: each impl's methods, and of `&mut I`
// its `RefSpec` specializations, the sized one's folds by `try_fold`. A
// toolchain whose std passes on another fails here, not as a wrong answer
// (#1986, #1991).
import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { root } from "./support";

const sysroot = Bun.spawnSync(["rustc", "--print", "sysroot"], { cwd: root }).stdout.toString().trim();
const library = join(sysroot, "lib/rustlib/src/rust/library");
const read = (file: string) => readFileSync(join(library, file), "utf8");

/** The methods an impl whose header starts `header` defines, its `fn`s and
 * what `impl_fold_via_try_fold!` makes. */
function methods(text: string, header: string): string[] {
  const at = text.indexOf(`\n${header}`);
  if (at < 0) throw new Error(`std has no \`${header}\`: read how it passes its iterator on again`);
  const body = text.slice(at + 1, text.indexOf("\n}\n", at) + 2);
  const fns = [...body.matchAll(/^ {4}(?:default )?fn (\w+)/gm)].map((m) => m[1]!);
  const via = [...body.matchAll(/impl_fold_via_try_fold! \{ (\w+) -> \w+ \}/g)].map((m) => m[1]!);
  return [...fns, ...via];
}

/** std's method names as `$forward`'s steps. */
const js: Record<string, string> = {
  next: "next",
  size_hint: "sizeHint",
  advance_by: "advanceBy",
  nth: "nth",
  fold: "fold",
  try_fold: "tryFold",
  last: "last",
  next_back: "back",
  advance_back_by: "advanceBackBy",
  nth_back: "nthBack",
  rfold: "rfold",
  try_rfold: "tryRfold",
  len: "len",
};

test("$lent and BOX_PASSES_ON pass on what std's &mut I and Box<I> pass on, of a sized and an unsized iterator", () => {
  const iterator = read("core/src/iter/traits/iterator.rs");
  const doubleEnded = read("core/src/iter/traits/double_ended.rs");
  const exact = read("core/src/iter/traits/exact_size.rs");
  const boxed = read("alloc/src/boxed/iter.rs");
  // `&mut I`: what its impls define, but its folds, which go to its
  // `RefSpec`: the sized specialization's pass them on, folds by `try_fold`
  // its `I`'s; the unsized, `default`, steps by `next` and `next_back`.
  const ref = [
    ...methods(iterator, "impl<I: Iterator + ?Sized> Iterator for &mut I {"),
    ...methods(doubleEnded, "impl<'a, I: DoubleEndedIterator + ?Sized> DoubleEndedIterator for &'a mut I {"),
    ...methods(exact, "impl<I: ExactSizeIterator + ?Sized> ExactSizeIterator for &mut I {"),
  ];
  const specs = { fold: "spec_fold", try_fold: "spec_try_fold", rfold: "spec_rfold", try_rfold: "spec_try_rfold" } as Record<string, string>;
  const sizedSpec = [
    ...methods(iterator, "impl<I: Iterator> IteratorRefSpec for &mut I {"),
    ...methods(doubleEnded, "impl<I: DoubleEndedIterator> DoubleEndedIteratorRefSpec for &mut I {"),
  ];
  // The unsized specialization's are `default` bodies over `next` and
  // `next_back`, none passed on.
  for (const header of ["impl<I: Iterator + ?Sized> IteratorRefSpec for &mut I {", "impl<I: DoubleEndedIterator + ?Sized> DoubleEndedIteratorRefSpec for &mut I {"]) {
    const text = header.includes("Double") ? doubleEnded : iterator;
    const body = text.slice(text.indexOf(header), text.indexOf("\n}\n", text.indexOf(header)));
    expect(body).not.toContain("(**self)");
  }
  const lent = ref.filter((name) => !(name in specs) || sizedSpec.includes(specs[name]!));
  const lentUnsized = ref.filter((name) => !(name in specs));
  // `Box<I>`: its impls' methods, its `last` by `BoxIter`, of a sized `I`
  // alone, as `BOX_PASSES_ON` lists them, the compiler's (#1601).
  const box = [
    ...methods(boxed, "impl<I: Iterator + ?Sized, A: Allocator> Iterator for Box<I, A> {"),
    ...methods(boxed, "impl<I: DoubleEndedIterator + ?Sized, A: Allocator> DoubleEndedIterator for Box<I, A> {"),
    ...methods(boxed, "impl<I: ExactSizeIterator + ?Sized, A: Allocator> ExactSizeIterator for Box<I, A> {"),
  ];
  expect(methods(boxed, "impl<I: Iterator, A: Allocator> BoxIter for Box<I, A> {")).toEqual(["last"]);
  const recognition = readFileSync(join(root, "src/lower/recognition/iterators.rs"), "utf8");
  const boxPassesOn = recognition.match(/pub\(in crate::lower\) const BOX_PASSES_ON: \[&str; \d+\] = \[([^\]]*)\];/);
  expect(boxPassesOn).not.toBeNull();
  expect([...boxPassesOn![1]!.matchAll(/"(\w+)"/g)].map((m) => m[1]!).sort()).toEqual([...new Set(box)].sort());
  const passedOn = readFileSync(join(root, "src/runtime/lent.js"), "utf8");
  const listed = (kind: string) => {
    const line = passedOn.match(new RegExp(`^ {2}${kind}: \\[([^\\]]*)\\],$`, "m"));
    if (!line) throw new Error(`src/runtime/lent.js's $passedOn has no ${kind}`);
    return [...line[1]!.matchAll(/"(\w+)"/g)].map((m) => m[1]!).sort();
  };
  const ours = (names: string[]) => [...new Set(names.filter((name) => name in js).map((name) => js[name]!))].sort();
  expect(listed("lent")).toEqual(ours(lent));
  expect(listed("lentUnsized")).toEqual(ours(lentUnsized));
  // Each std method named is one `$forward` knows, or `is_empty`, which
  // asks `len`.
  expect([...ref, ...box].filter((name) => !(name in js) && name !== "is_empty")).toEqual([]);
});
