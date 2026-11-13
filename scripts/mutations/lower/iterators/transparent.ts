// Mutations of src/lower/iterators/transparent.rs (ADR 0093): which chains
// are transparent (#1673), each answer held by its count.
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "transparent-crate-iterator-unseen",
    breaks: "an `Iterator` of the crate's in a chain is taken as no reason, so `Counter(0).map(..)` counts as held",
    file: "src/lower/iterators/transparent.rs",
    find: "args.types().any(crate_iterator)",
    replace: "args.types().any(crate_iterator) && false",
    tests: ["test/decisions.test.ts", "-t", "each chain is counted by what keeps it from being transparent"],
  },
  {
    name: "transparent-held-unseen",
    breaks: "a chain that starts on an iterator held apart, `evens(&v).count()`, is taken as starting on another source",
    file: "src/lower/iterators/transparent.rs",
    find: "        } else if source\n            .is_some_and(|s| is_iterator(thir[s].ty) && !is_plain_source(",
    replace: "        } else if false && source\n            .is_some_and(|s| is_iterator(thir[s].ty) && !is_plain_source(",
    tests: ["test/decisions.test.ts", "-t", "each chain is counted by what keeps it from being transparent"],
  },
  {
    name: "transparent-any-source-plain",
    breaks: "a chain on any source is taken as on an array, so a `BTreeSet`'s `iter().sum()` is admitted",
    file: "src/lower/iterators/transparent.rs",
    find: "    matches!(ty.kind(), ty::Array(..) | ty::Slice(_))\n",
    replace: "    true || matches!(ty.kind(), ty::Array(..) | ty::Slice(_))\n",
    tests: ["test/decisions.test.ts", "-t", "each chain is counted by what keeps it from being transparent"],
  },
  {
    name: "transparent-escapes-unseen",
    breaks: "an iterator a chain gives back, `evens`' `v.iter().filter(..)`, is taken as consumed where it's made",
    file: "src/lower/iterators/transparent.rs",
    find: "        } else if is_iterator(thir[root].ty) && !thir[root].span.is_desugaring(DesugaringKind::ForLoop) {\n",
    replace: "        } else if false && is_iterator(thir[root].ty) {\n",
    tests: ["test/decisions.test.ts", "-t", "each chain is counted by what keeps it from being transparent"],
  },
  {
    name: "transparent-for-loop-escapes",
    breaks: "a `for` loop's chain is taken as escaping, so `for x in v.iter().rev()` isn't admitted",
    file: "src/lower/iterators/transparent.rs",
    find: "        } else if is_iterator(thir[root].ty) && !thir[root].span.is_desugaring(DesugaringKind::ForLoop) {\n",
    replace: "        } else if is_iterator(thir[root].ty) {\n",
    tests: ["test/decisions.test.ts", "-t", "each chain is counted by what keeps it from being transparent"],
  },
  {
    name: "transparent-item-drops-unseen",
    breaks: "an item with a destructor, `names.into_iter().map(..)`'s `String`, is taken as dropping nothing",
    file: "src/lower/iterators/transparent.rs",
    find: "    let item = recognition.iterator_item(made.peel_refs()).is_some_and(needs_drop);\n",
    replace: "    let item = recognition.iterator_item(made.peel_refs()).is_some_and(needs_drop) && false;\n",
    tests: ["test/decisions.test.ts", "-t", "each chain is counted by what keeps it from being transparent"],
  },
  {
    name: "transparent-capture-drops-unseen",
    breaks: "a closure's capture with a destructor, a `move` closure's `String`, is taken as dropping nothing",
    file: "src/lower/iterators/transparent.rs",
    find: "                ty::Closure(_, closure_args) => closure_args.as_closure().upvar_tys().iter().any(needs_drop),\n",
    replace: "                ty::Closure(..) => false,\n",
    tests: ["test/decisions.test.ts", "-t", "each chain is counted by what keeps it from being transparent"],
  },
  {
    name: "transparent-loops-next-counted",
    breaks: "a `for` loop's own `next` is counted as a chain of its own, held",
    file: "src/lower/iterators/transparent.rs",
    find: "    for &root in links.iter().filter(|&&e| !inner.contains(&e) && !loops_next(e)) {\n",
    replace: "    for &root in links.iter().filter(|&&e| !inner.contains(&e)) {\n",
    tests: ["test/decisions.test.ts", "-t", "each chain is counted by what keeps it from being transparent"],
  },
  {
    name: "transparent-each-link-a-chain",
    breaks: "each link of a chain is counted as a chain of its own",
    file: "src/lower/iterators/transparent.rs",
    find: "    for &root in links.iter().filter(|&&e| !inner.contains(&e) && !loops_next(e)) {\n",
    replace: "    for &root in links.iter().filter(|&&e| !loops_next(e)) {\n",
    tests: ["test/decisions.test.ts", "-t", "each chain is counted by what keeps it from being transparent"],
  },
];
