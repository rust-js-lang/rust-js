// Mutations of src/lower/body_queries/ranges/known.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "join-unbound-forgets",
    breaks: "a range bound by no length, `let mut s = 0`'s, joined with one below `xs.len()` forgets the bound, so a counter past its loop isn't known below the length",
    file: "src/lower/body_queries/ranges/known.rs",
    find: "            None => Some(Below { c: 0, b: self.hi, s: 0 }),\n",
    replace: "            None => None,\n",
    tests: ["test/lowering-numbers.test.ts", "-t", "a signed index tested not negative"],
  },
  {
    name: "ranges-within-loose",
    breaks: "an index at most the length is taken below it, so `items[i]` where `i <= items.len()` reads `undefined`",
    file: "src/lower/body_queries/ranges/known.rs",
    find: "        self.c <= 1 << self.s && self.b <= -(1 << self.s)",
    replace: "        self.c <= 1 << self.s && self.b <= 0",
    tests: ["test/lowering-numbers.test.ts", "-t", "integer ranges show"],
  },
  {
    name: "pop-as-push",
    breaks: "a `pop` is taken as one more item, so `v[n - 1]` of the length before it reads `undefined`",
    file: "src/lower/body_queries/ranges/known.rs",
    find: "                bound.b -= bound.c * by;",
    replace: "                bound.b -= bound.c * by.abs();",
    tests: ["test/lowering-functions.test.ts", "-t", "only calls reach"],
  },
  {
    name: "bound-never-lowest",
    breaks: "a bound is never made over its least denominator, so a loop that halves `siftUp`'s index never settles and `heap[index]` is checked",
    file: "src/lower/body_queries/ranges/known.rs",
    find: "        while self.s > 0 && self.c % 2 == 0 && self.b % 2 == 0 {",
    replace: "        while false && self.s > 0 && self.c % 2 == 0 && self.b % 2 == 0 {",
    tests: ["test/lowering-functions.test.ts", "-t", "only calls reach"],
  },
  {
    name: "product-negative-zero-exact",
    breaks: "a product the ranges show in range is JS's own `*` though it can be `-4 * 0`, JS's `-0`, which an `f64` made of it shows",
    file: "src/lower/body_queries/ranges/known.rs",
    find: "        zero(self) && other.lo < 0 || zero(other) && self.lo < 0\n",
    replace: "        false\n",
    tests: ["test/corpus.test.ts", "-t", "^product_sign.rs$"],
  },
  {
    name: "push-of-unknown-length-unknown",
    breaks: "a `push` of a `Vec` whose length isn't known leaves it unknown, so the length read after it may be 0, and `properties[prevLength - 1]` is `$index(properties, (prevLength - 1) >>> 0)`",
    file: "src/lower/body_queries/ranges/known.rs",
    find: "            None if by > 0 => _ = self.lengths.insert(slice.clone(), by),\n",
    replace: "            None if by > 0 => {}\n",
    tests: ["test/lowering-numbers.test.ts", "-t", "a length held in a variable"],
  },
  {
    name: "at-least-one-short",
    breaks: "a value at least 1 and at most a length shows nothing of the length, so `keys[0]` past `length === 1` keeps `$index`",
    file: "src/lower/body_queries/ranges/known.rs",
    find: "        if least > 0 {\n",
    replace: "        if least > 1 {\n",
    tests: ["test/lowering-numbers.test.ts", "-t", "a length held in a variable"],
  },
];
