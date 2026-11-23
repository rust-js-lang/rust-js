// Mutations of crates/rust-js-lower/src/iterators/back_facts.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [

  {
    name: "made-from-unread",
    breaks: "what a call is given isn't asked, only what it makes, so a map made in place where it's stepped, `count().map(f).rfold(..)`, built at its step, is never made knowing its back, and is refused (#851)",
    file: "crates/rust-js-lower/src/iterators/back_facts.rs",
    find: "            && (self.made_into.iter().chain(&self.made_from)).any(|&(local, made)| {\n",
    replace: "            && (self.made_into.iter()).any(|&(local, made)| {\n",
    tests: ["test/corpus.test.ts", "-t", "^backs_given_behind_impls.rs$"],
    snapshots: true,
  },
  {
    name: "bare-array-iterator-trusted",
    breaks: "a variable of an array's iterator's type alone is taken to have a back, though it may hold the array, so a chain of one `w` is `$lazyChain($iter(v), w)`, the array itself where a JS iterator is written, which once threw `it.back is not a function` and now steps only as the runtime tolerates an array there (#851, #1734)",
    file: "crates/rust-js-lower/src/iterators/back_facts.rs",
    find: "                    && (wrapped || !self.steps_an_array(ty))\n",
    replace: "                    && true\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_chain.rs$"],
    snapshots: true,
  },
  {
    name: "compiled-variable-trusted",
    breaks: "a variable holding std's compiled struct is taken to have a back by its type, though it has one only given it, so a `rev()` of what a function returns throws `it.back is not a function` (#856, #851)",
    file: "crates/rust-js-lower/src/iterators/back_facts.rs",
    find: "                    && !self.compiled_struct(ty, value)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(returned_impl_ends|chain_under_a_held_map_from_its_back).rs$"],
  },

  {
    name: "char-indices-back-unknown",
    breaks: "a `char_indices()` isn't asked whether it knows its back as a `chars()` is, so generic code's `nth_back` of one is refused",
    file: "crates/rust-js-lower/src/iterators/back_facts.rs",
    find: "        (text.is_str_chars(ty) || text.is_char_indices(ty)).then(|| self.chars_asked_whole(ty))\n",
    replace: "        text.is_str_chars(ty).then(|| self.chars_asked_whole(ty))\n",
    tests: ["test/lowering-strings.test.ts", "-t", "each one thing more asked of a chars"],
  },
  {
    name: "chars-cloned-unseen",
    breaks: "a `chars()` the crate clones is JS's own string iterator, which has no `clone()`",
    file: "crates/rust-js-lower/src/iterators/back_facts.rs",
    find: "        self.krate.chars_whole\n            || self\n",
    replace: "        self.krate.chars_whole\n            || false && self\n",
    tests: ["test/lowering-strings.test.ts", "-t", "each one thing more asked of a chars"],
  },
  {
    name: "knowing-in-its-maker-unread",
    breaks: "a `map` generic code makes of its own type parameter's iterator, where its hint is asked, is JS's own, `(0, None)` (#1240)",
    file: "crates/rust-js-lower/src/iterators/back_facts.rs",
    find: "        } else if hinted.knowing_in.contains(&(owner, ty)) {\n",
    replace: "        } else if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^capacity_of_chains_of_exact_size_adapters.rs$"],
    snapshots: true,
  },
  {
    name: "counted-in-its-maker-unread",
    breaks: "a `take(2)` generic code makes of its own type parameter's iterator, where its hint is asked, is JS's own `.take(2)`, `(0, None)` (#1240)",
    file: "crates/rust-js-lower/src/iterators/back_facts.rs",
    find: "        let answer = if hinted.counted_in.contains(&(owner, ty)) {\n",
    replace: "        let answer = if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_made_in_generic_code_asked_its_hint\\.rs$"],
  },
  {
    name: "hint-typed-unread",
    breaks: "what's made knowing its hint wherever it's made, an adapter given to generic code that asks its hint, isn't, so a filter of a `Vec`'s items is JS's own, whose hint is `(0, None)` (#1236, #2209)",
    file: "crates/rust-js-lower/src/iterators/back_facts.rs",
    find: "        } else if hinted.is_typed(self.tcx, owner, ty) {\n",
    replace: "        } else if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^std_adapter_given_to_generic_code_that_asks_its_hint.rs$"],
  },
  {
    name: "hint-made-stepped-from-its-back",
    breaks: "what's made knowing its hint by the function that makes it is said to be stepped from its back, so a flatten of the crate's iterators, which have none, beside another of its type whose hint is asked, is `$flattenEnds`, which throws `an iterator stepped as an array's` (#2209)",
    file: "crates/rust-js-lower/src/iterators/back_facts.rs",
    find: "        let answer = if (self.krate.iterators.back_stepped).is_typed(self.tcx, self.frame.owner, ty) {\n",
    replace: "        let answer = if (self.krate.iterators.back_stepped).is_typed(self.tcx, self.frame.owner, ty) || self.krate.iterators.hinted.knowing_in.contains(&(self.tcx.typeck_root_def_id(self.frame.owner), ty)) {\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_made_knowing_its_hint_asks_no_back.rs$"],
  },
];
