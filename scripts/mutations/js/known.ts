// Mutations of src/js/known.rs (ADR 1127).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "nothing-forgotten",
    breaks: "`--without-known` forgets nothing: each of the net's variants is the JS a build makes (ADR 1127)",
    file: "src/js/known.rs",
    find: "        knowns.iter().fold(0, |bits, known| bits | known.bit()),\n",
    replace: "        knowns.iter().fold(0, |bits, _| bits),\n",
    tests: ["test/tidy-stages.test.ts", "-t", "a case's JS without a fact a reader found"],
  },
  {
    name: "iterator-fact-only-permits",
    breaks:
      "`Known::Iterator` is taken to only permit, so the net forgets it: a `while let` of a `by_ref()`'s steps, not known to be a JS iterator, is stepped as an array and never ends its loop as Rust does (ADR 1127)",
    file: "src/js/known.rs",
    find: "            Known::InPlace\n            | Known::Iterator\n",
    replace: "            Known::Iterator => Role::Permits,\n            Known::InPlace\n",
    tests: ["test/tidy-stages.test.ts", "-t", "by_ref_steps after each"],
  },
];
