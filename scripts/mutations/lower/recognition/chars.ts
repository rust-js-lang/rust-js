// Mutations of src/lower/recognition/chars.rs (ADR 1117).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "chars-given-to-helpers-unseen",
    breaks:
      "a `Chars` given to the crate's own helper or generic code is JS's own string iterator, so the helper's `next_back()` is refused and generic code's `size_hint()` is `(0, None)`",
    file: "src/lower/recognition/chars.rs",
    find: "        if std_crate(tcx, callee.krate).is_none() {\n",
    replace: "        if false {\n",
    tests: ["test/lowering-strings.test.ts", "-t", "each one thing more asked of a chars"],
  },
  {
    name: "chars-held-unrevealed",
    breaks: "a `next_back()` of another crate's `impl DoubleEndedIterator` hiding a `Chars` isn't seen to step a `Chars`, and is refused",
    file: "src/lower/recognition/chars.rs",
    find: "            .map(|part| self.reveal(part))\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "two crates: library_chars_behind_impl_trait"],
  },
  {
    name: "chars-rest-unseen",
    breaks: "a `Chars` asked its `as_str()` is JS's own string iterator, which has no rest to give",
    file: "src/lower/recognition/chars.rs",
    find: '        if name == "as_str"\n',
    replace: '        if false && name == "as_str"\n',
    tests: ["test/lowering-strings.test.ts", "-t", "each one thing more asked of a chars"],
  },
  {
    name: "chars-hint-unseen",
    breaks: "a `Chars` asked its `size_hint()` is JS's own string iterator, whose hint is `(0, None)`",
    file: "src/lower/recognition/chars.rs",
    find: '(name == "size_hint" || double_ended || ',
    replace: '(false || double_ended || ',
    tests: ["test/lowering-strings.test.ts", "-t", "each one thing more asked of a chars"],
  },
  {
    name: "chars-back-unseen",
    breaks: "a `Chars` stepped from its back, or what holds one, a `Fuse`, is JS's own string iterator, which has no back",
    file: "src/lower/recognition/chars.rs",
    find: '(name == "size_hint" || double_ended || matches!',
    replace: '(name == "size_hint" || false || matches!',
    tests: ["test/lowering-strings.test.ts", "-t", "each one thing more asked of a chars"],
  },
  {
    name: "chars-debug-unseen",
    breaks: "a `Chars` shown by `{:?}` is JS's own string iterator, whose `Debug` shows none of what it has left",
    file: "src/lower/recognition/chars.rs",
    find: "                .any(|clause| Some(clause.def_id()) == debug && holds(clause.self_ty().skip_binder()))\n",
    replace: "                .any(|clause| false && Some(clause.def_id()) == debug && holds(clause.self_ty().skip_binder()))\n",
    tests: ["test/corpus.test.ts", "-t", "^pretty_debug_of_std_iterators.rs$"],
  },
  {
    name: "chars-public-fields-unseen",
    breaks: "a library's public struct whose public field holds a `Chars` gives another crate JS's own string iterator, whose back, rest and hint it can't ask",
    file: "src/lower/recognition/chars.rs",
    find: "            ty::Adt(adt, args) if adt.did().is_local() => (adt.all_fields())\n",
    replace: "            ty::Adt(adt, args) if false && adt.did().is_local() => (adt.all_fields())\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_chars_in_a_public_field"],
  },
  {
    name: "chars-opaque-unrevealed",
    breaks: "a library's `impl DoubleEndedIterator` hiding a `Chars` gives another crate JS's own string iterator, whose back it can't step",
    file: "src/lower/recognition/chars.rs",
    find: "            ty::Alias(_, alias) if let ty::Opaque { def_id } = alias.kind => {\n",
    replace: "            ty::Alias(_, alias) if false && let ty::Opaque { def_id } = alias.kind => {\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_chars_behind_impl_trait"],
  },
];
