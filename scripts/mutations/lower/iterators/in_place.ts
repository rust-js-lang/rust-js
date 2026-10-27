// Mutations of src/lower/iterators/in_place.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "fuse-of-drops-elided",
    breaks: "a `fuse()` stepped in place given its source's drop is elided with it, so what a `Vec` of items with a destructor didn't give is never dropped (ADR 0983)",
    file: "src/lower/iterators/in_place.rs",
    find: "        _ if fused && given.len() <= 3 && given.first().is_some_and(is_js_iterator) => given[0].clone(),\n",
    replace: "        _ if fused && given.first().is_some_and(is_js_iterator) => given[0].clone(),\n",
    tests: ["test/corpus.test.ts", "-t", "^owned_items_through_fuse.rs$"],
  },
  {
    name: "iter-eager-lent-root-kept",
    breaks: "a chain on `$lent($iter(xs))` just made stays a JS iterator's",
    file: "src/lower/iterators/in_place.rs",
    find: "js::ExprKind::Call(lent, of) if lent.helper_name() == Some(\"$lent\") && of.len() == 1 => &of[0],",
    replace: "js::ExprKind::Call(lent, of) if lent.helper_name() == Some(\"$lent\") && of.len() == 1 => e,",
    tests: ["test/lowering-matches.test.ts", "-t", "let-else from MIR is a guard"],
  },

  {
    name: "does-f64-max-given-acts",
    breaks: "`$f64Max` given as it is acts, so a chain it folds stays lazy",
    file: "src/lower/iterators/in_place.rs",
    find: "        js::ExprKind::Var(_) => !f.helper_name().is_some_and(crate::runtime::is_pure),\n",
    replace: "",
    tests: ["test/traits.test.ts","-t","MIR dictionaries are explicit"],
  },
  {
    name: "into-iter-for-kept",
    breaks: "a `for` of a generic `into_iter()` steps `$intoIter(xs)`, where a `for` steps `xs` as it is",
    file: "src/lower/iterators/in_place.rs",
    find: "        Some(\"$intoIter\") if by_for => given[0].clone(),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^generic_into_iter_values.rs$"],
    snapshots: true,
  },
  {
    name: "fuse-kept-in-place",
    breaks: "`fuse()` of a JS iterator stepped till it ends keeps its `iterator_fuse` call, which steps the same",
    file: "src/lower/iterators/in_place.rs",
    find: "        _ if fused && given.len() <= 3 && given.first().is_some_and(is_js_iterator) => given[0].clone(),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^fused_iterator.rs$"],
    snapshots: true,
  },
  {
    name: "consumed-iterator-keeps-hint",
    breaks: "an iterator of the crate's stepped till it ends keeps a `size_hint` nothing asks, `$iterator(x, next, false, undefined, undefined, hint)`",
    file: "src/lower/iterators/in_place.rs",
    find: "    if callee.helper_name() == Some(\"$iterator\") && given.len() >= 6 {\n",
    replace: "    if false && callee.helper_name() == Some(\"$iterator\") && given.len() >= 6 {\n",
    tests: ["test/corpus.test.ts", "-t", "^(fused_iterators|double_ended_iterators).rs$"],
    snapshots: true,
  },
];
