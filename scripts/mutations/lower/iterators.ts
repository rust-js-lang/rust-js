// Mutations of src/lower/iterators.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "into-iter-named-not-held",
    breaks:
      "an `IntoIterator` type parameter `impl Is<I>` names is made a JS iterator by `Iterator.from` where its caller gave its own value, a crate's iterator struct (#966)",
    file: "src/lower/iterators.rs",
    find: "        && bound_as_iterator(tcx, id, param)\n        && named_by_another_bound(tcx, id, param)\n",
    replace: "        && false\n",
    tests: ["test/corpus.test.ts", "-t", "^into_iterator_named_by_another_bound.rs$"],
  },
  {
    name: "naming-parameter-of-into-iter-boxed",
    breaks:
      "`impl Is<I>` of an `IntoIterator` held as it is isn't given as it is, so a `Vec`'s iterator of items whose drop may panic given for it is wrapped in its slice's steps, `$slicing(it, drop)`, which `I`'s dictionary steps already (#966, #1109)",
    file: "src/lower/iterators.rs",
    find: "                        && into_iter_held_as_it_is(tcx, id, Ty::new_param(tcx, p.index, p.name)) =>\n",
    replace: "                        && false =>\n",
    tests: ["test/corpus.test.ts", "-t", "^into_iterator_named_by_another_bound.rs$"],
    snapshots: true,
  },
  {
    name: "reversal-of-range-chain-as-items",
    breaks:
      "a map of a range stepped from its end, made in place, is read as its source's map, which steps the range forward: `Some(2)` where Rust gives `Some(9)` (ADR 0987)",
    file: "src/lower/iterators.rs",
    find: "            _ if is_stepped_items(e) => true,\n",
    replace:
      "            _ if is_stepped_items(e) || steps(e).is_some() => true,\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "a map of a reversed range given where its length is asked",
    ],
  },
  {
    name: "made-array-helper-lazy",
    breaks:
      "`v.windows(2).map(f).collect()` from MIR stays `Array.from(Iterator.from($windows(v, 2)).map(f))`",
    file: "src/lower/iterators.rs",
    find: "        js::ExprKind::Call(callee, _) if matches!(callee.kind, js::ExprKind::Var(_)) => fresh(e),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(calculator|map_set_methods).rs$"],
    snapshots: true,
  },
  {
    name: "mir-object-keys-iterated",
    breaks:
      "`Object.keys(o)`, an array, is taken as a JS iterator and stepped as one",
    file: "src/lower/iterators.rs",
    find: '            js::ExprKind::Var(v) if v == "Object" => false,\n',
    replace: "",
    tests: ["test/lowering-loops.test.ts", "-t", "a for over Object.keys"],
  },
  {
    name: "mir-adapter-not-js-iterator",
    breaks:
      "an adapter of a JS iterator, `.map(f)` of a range's, is made one again, `Iterator.from(..)`",
    file: "src/lower/iterators.rs",
    find: '            _ if matches!(name.as_str(), "map" | "filter" | "flatMap" | "take" | "drop") => is_js_iterator(object),\n',
    replace: "",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "an adapter of a JS iterator is not made one again",
    ],
  },
  {
    name: "given-as-it-is-not-held",
    breaks:
      "an adapter's `I`, which its type holds, is a JS iterator, so `Doubled { inner: 0..4 }`'s range is stepped as one",
    file: "src/lower/iterators.rs",
    find: "        has(tcx.type_of(imp).instantiate_identity().skip_normalization())\n            || (tcx.associated_items(imp)",
    replace: "        (has(tcx.type_of(imp).instantiate_identity().skip_normalization()) && false)\n            || (tcx.associated_items(imp)",
    tests: ["test/corpus.test.ts", "-t", "^iterator_adapters.rs$"],
  },
  {
    name: "given-as-it-is-not-given-back",
    breaks:
      "`doubled(0..4)`, whose result holds its `I`, converts the range to a JS iterator its caller steps as a range",
    file: "src/lower/iterators.rs",
    find: "            has(sig.output()) || meet || inside\n",
    replace: "            false || meet || inside\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_adapters.rs$"],
  },
  {
    name: "naming-parameter-not-as-it-is",
    breaks:
      "`impl Is<I>` of an iterator given as it is isn't given as it is, so a `Vec`'s iterator of items whose drop may panic given for it is wrapped in its slice's steps, `$slicing(it, drop)`, which `I`'s dictionary steps already (#966, #1109)",
    file: "src/lower/iterators.rs",
    find: "        return names_given_as_it_is(tcx, id, param);\n",
    replace: "        return false;\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_named_by_another_bound.rs$"],
    snapshots: true,
  },
  {
    name: "given-as-it-is-not-in-input",
    breaks:
      "`second(Wrap(7..10))`'s range, in what's given, is stepped as a JS iterator",
    file: "src/lower/iterators.rs",
    find: "            let inside =\n                (inputs.iter()).any(|&input| !itself(crate::lower::recognition::iterator_held(input)) && has(input));\n",
    replace: "            let inside = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_adapters.rs$"],
  },
  {
    name: "given-as-it-is-projection",
    breaks:
      "`all<I>(i) -> Vec<I::Item>`, which gives back only its items, takes its `I` as it is, with a dictionary",
    file: "src/lower/iterators.rs",
    find: "            Some(ty::Alias(..)) => walk.skip_current_subtree(),\n            Some(ty::Param(p)) if p.index == param.index => return true,\n",
    replace: "            Some(ty::Param(p)) if p.index == param.index => return true,\n",
    tests: ["test/corpus.test.ts", "-t", "^associated_types.rs$"],
    snapshots: true,
  },
  {
    name: "fresh-iter-sliced",
    breaks:
      "what's left of a just-made `$iter(items)` is `it.items.slice(it.at, it.end)`, not `items`",
    file: "src/lower/iterators.rs",
    find: '        js::ExprKind::Call(callee, args) if callee.helper_name() == Some("$iter") => match args.as_slice() {\n',
    replace: '        js::ExprKind::Call(callee, args) if false && callee.helper_name() == Some("$iter") => match args.as_slice() {\n',
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^iter_cycle.rs$"],
  },
  {
    name: "cycle-wrapped",
    breaks:
      "a `$cycle`'s generator is wrapped in `Iterator.from(..)`, though it's a JS iterator already",
    file: "src/lower/iterators.rs",
    find: '    "$cycle",\n',
    replace: '    "$cycled",\n',
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^iter_cycle.rs$"],
  },
  {
    name: "stepped-back-as-boxed",
    breaks:
      "`$iterator(it, next, nextBack)` reads its `nextBack` as whether its `Some` is boxed",
    file: "src/lower/iterators.rs",
    find: "        .map(|(at, arg)| {\n            arg.unwrap_or_else(|| match at {\n                0 => Expr::bool(false),\n                _ => Expr::js_undefined(),\n            })\n        })\n",
    replace: "        .filter_map(|(_, arg)| arg)\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^generic_double_ended.rs$"],
  },
  {
    name: "lazy-chain-not-iterator",
    breaks:
      "`$lazyChain(a, b)`, a generator, is taken for what isn't a JS iterator, and wrapped in `Iterator.from`",
    file: "src/lower/iterators.rs",
    find: '    "$lazyChain",\n',
    replace: '    "$lazyChained",\n',
    tests: ["test/corpus.test.ts", "-t", "^adapter_drops.rs$"],
    snapshots: true,
  },
  {
    name: "iterator-read-as-js",
    breaks:
      "the crate's iterator given to `I: Iterator + Clone` is its JS iterator, which its `Clone` and `Debug` read as its value",
    file: "src/lower/iterators.rs",
    find: "    held || signature || read\n",
    replace: "    held || signature\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_read_as_itself.rs$"],
  },
  {
    name: "extension-bound-read",
    breaks:
      "a `DoubleEndedIterator` bound is taken for one that reads its iterator as its own type, which is then given as it is",
    file: "src/lower/iterators.rs",
    find: "        && !super::recognition::is_iterator_extension(tcx, tr)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^associated_double_ended.rs$"],
    snapshots: true,
  },
  {
    name: "extension-bound-not-given-as-is",
    breaks:
      "an `I: DoubleEndedIterator` a type holds is a JS iterator in its impl, which its caller's range isn't",
    file: "src/lower/iterators.rs",
    find: "        .any(|&tr| tr == iterator || super::recognition::implies_iterator(tcx, tr))\n",
    replace: "        .any(|&tr| tr == iterator)\n",
    tests: ["test/corpus.test.ts", "-t", "^extension_bound_iterators.rs$"],
  },
  {
    name: "closing-not-an-iterator",
    breaks:
      "`$closing` is taken for what isn't a JS iterator, written inside `Iterator.from`, and its chain not made eager",
    file: "src/lower/iterators.rs",
    find: '    "$closing",\n',
    replace: '    "$peekable",\n',
    tests: ["test/corpus.test.ts", "-t", "^adapter_drops.rs$"],
    snapshots: true,
  },
  {
    name: "std-provided-self-given-by-trait",
    breaks:
      "a compiled std provided method's `Self`, `peekable`'s, isn't given as it is, so `@rust-js/std`'s `iterator_peekable` takes no dictionary for it, and a caller's generic iterator steps nothing",
    file: "src/lower/iterators.rs",
    find: "    ) && (tcx.trait_of_assoc(id).is_none() || super::recognition::is_compiled_std(tcx, id))\n",
    replace: "    ) && tcx.trait_of_assoc(id).is_none()\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "into-iter-never-held",
    breaks:
      "no `IntoIterator`'s `IntoIter` is held as it is, so a range is given as a JS iterator and stepped as a range",
    file: "src/lower/iterators.rs",
    find: "    read || named || held\n",
    replace: "    false\n",
    tests: ["test/corpus.test.ts", "-t", "^into_iter_held_as_it_is.rs$"],
  },
  {
    name: "into-iter-param-made-js-iterator",
    breaks:
      "a type parameter whose `IntoIter` is held as it is is made a JS iterator by its caller",
    file: "src/lower/iterators.rs",
    find: "    if into_iter_held_as_it_is(tcx, id, Ty::new_param(tcx, param.index, param.name)) {\n        return true;\n    }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^into_iter_held_as_it_is.rs$"],
  },
  {
    name: "into-iter-held-in-struct-a-js-iterator",
    breaks:
      "an `IntoIterator` whose `IntoIter` a function holds in a struct of the crate's, `Pair<I::IntoIter>`, is given a JS iterator, which a compiled `Fuse` then holds where its callers read the struct",
    file: "src/lower/iterators.rs",
    find: "    read || named || held\n",
    replace: "    read || named\n",
    tests: ["test/corpus.test.ts", "-t", "^fused_iterators.rs$"],
  },
  {
    name: "dyn-keeps-own-fold",
    breaks:
      "a `Box<dyn Iterator>` of the crate's iterator folds by its own `fold`, where std's `Box` folds by `next`",
    file: "src/lower/iterators.rs",
    find: "        given[6] = Expr::js_undefined();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^own_fold_and_nth.rs$"],
  },
  {
    name: "sliced-not-made",
    breaks:
      "a slice's range in bounds, `items.slice(0, 3)`, isn't an array made here, so its `to_vec()` copies it again, `.slice(0, 3).slice()` (ADR 0806)",
    file: "src/lower/iterators.rs",
    find: "        _ if e.facts.has(js::Known::Sliced) => true,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "slice_ranges_known"],
    snapshots: true,
  },
  {
    name: "alias-into-iter-not-held",
    breaks:
      "generic code given `I::Item: IntoIterator` iterates each item as a JS iterator, which an `Option`'s `T | undefined` isn't (#278)",
    file: "src/lower/iterators.rs",
    find: "    if matches!(*into.kind(), ty::Alias(_, ty::AliasTy { kind: ty::Projection { def_id }, .. }) if def_id != into_iter)\n    {\n",
    replace:
      "    if false && matches!(*into.kind(), ty::Alias(_, ty::AliasTy { kind: ty::Projection { def_id }, .. }) if def_id != into_iter)\n    {\n",
    tests: ["test/corpus.test.ts", "-t", "^into_iter_of_items.rs$"],
  },
  {
    name: "held-into-iter-param-not-given",
    breaks:
      "`U` of `I::Item: IntoIterator<IntoIter = U>` is a JS iterator, so `Flatten`'s `next` gives `FlattenCompat` a generic `{ next }` of what's held as it is",
    file: "src/lower/iterators.rs",
    find: "    if is_held_into_iter(tcx, id, param) {\n",
    replace: "    if false && is_held_into_iter(tcx, id, param) {\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "impl-trait-hides-held-into-iter",
    breaks:
      "an `impl Iterator` a function gives, hiding std's `Chain` of what it's given, isn't read as holding a `J::IntoIter`, so `J` is given as a JS iterator, `chain($iterator(countdown, next), tmp, drop)`, with dictionaries made in the function, not given as it is with its dictionary (ADR 0920)",
    file: "src/lower/iterators.rs",
    find: "        .any(|ty| holds(ty) || hidden(ty));\n",
    replace: "        .any(|ty| holds(ty) || false && hidden(ty));\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^generic_chain_of_into.rs$"],
  },
  {
    name: "stepped-slices-length-kept",
    breaks:
      "a slice's iterator asked its length too, `it.len()`, is made the array's own `values()`, which has none, a TypeError (rd270)",
    file: "src/lower/iterators.rs",
    find: "        if declared != 1 || steps == 0 || js::mentions_in(body, &name) != steps {\n",
    replace: "        if declared != 1 || steps == 0 {\n",
    tests: ["test/lowering-iterators.test.ts", "-t", "only stepped"],
  },
  {
    name: "known-iterator-made-again",
    breaks:
      "a variable its type says is a JS iterator, a held `take_while`'s, is made one again by `Iterator.from`, whose `[Symbol.iterator]` makes it close as it's done, dropping what's left before the statement ends (generated program 12454000)",
    file: "src/lower/iterators.rs",
    find: "    if e.facts.has(js::Known::Iterator) {\n        return true;\n    }\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^step_by_of_a_call_holds_its_items.rs$",
    ],
  },
  {
    name: "lent-param-meets-unseen",
    breaks:
      "a generic function's `&mut I` and another `I`, `replace_in(it: &mut I, next: I)`'s, leave `I` a JS iterator, so `*it = next` is refused (#537)",
    file: "src/lower/iterators.rs",
    find: "&& given.count() > 1;",
    replace: "&& given.count() > 2;",
    tests: ["test/corpus.test.ts", "-t", "^iterator_values_meet.rs$"],
  },
  {
    name: "extension-step-held",
    breaks:
      "`v.iter().map(f).rfold(..)` takes the `map()` as std's struct, which `DoubleEndedIterator`'s methods steps where it's made",
    file: "src/lower/iterators.rs",
    find: "        .filter_map(|clause| clause.as_trait_clause())\n",
    replace:
      "        .filter_map(|clause| clause.as_trait_clause())\n        .filter(|_| !tcx.trait_of_assoc(callee).is_some_and(|tr| super::recognition::is_iterator_extension(tcx, tr)))\n",
    snapshots: true,
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(extension_map_rfold|generic_map_folded_and_rfolded|map_stepped_from_its_back|exact_size_len).rs$",
    ],
  },
  {
    name: "unread-map-by-its-shape",
    breaks: "an unread map is no statement by its shape alone, `x.map(f)`, so an unread `collect()` of an array's eager `map` never runs its closure (#805)",
    file: "src/lower/iterators.rs",
    find: "        e.facts.has(js::Known::Iterator) && method == \"map\" && f.reads_only_vars()\n",
    replace: "        method == \"map\" && f.reads_only_vars()\n",
    tests: ["test/corpus.test.ts", "-t", "^unread_map_of_a_held_iterator.rs$"],
  },
  {
    name: "into-iter-held-in-a-library-type-unseen",
    breaks:
      "whether a function holds what an `IntoIterator` makes as it is asks of a type of the crate compiling, so a library's `both` returning its own `Both<I::IntoIter, J::IntoIter>` takes its dictionaries and its consumer gives none, `IIntoIterator.intoIter is not a function` (#1025)",
    file: "src/lower/iterators.rs",
    find: "                if adt.did().krate == id.krate || super::recognition::is_compiled_std_type(tcx, part.expect_ty()) =>\n",
    replace: "                if adt.did().is_local() || super::recognition::is_compiled_std_type(tcx, part.expect_ty()) =>\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_into_iter_held_as_it_is"],
  },
  {
    name: "into-iter-hidden-by-a-library-unseen",
    breaks:
      "whether a function holds what an `IntoIterator` makes as it is, behind an `impl Iterator` it gives, asks of an opaque type of the crate compiling, so a library's `chained`'s consumer gives no dictionary its JS takes (#1025)",
    file: "src/lower/iterators.rs",
    find: "            )) if def_id.krate == id.krate => holds(tcx.type_of(def_id).instantiate(tcx, args).skip_normalization()),\n",
    replace: "            )) if def_id.is_local() => holds(tcx.type_of(def_id).instantiate(tcx, args).skip_normalization()),\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_into_iter_held_as_it_is"],
  },
  {
    name: "owned-range-copied-twice",
    breaks: "an owned copy of a range, `v[..n].to_vec()`, is the part copied again, `$slice(v, 0, n).slice()`, two copies where one is",
    file: "src/lower/iterators.rs",
    find: "        js::ExprKind::Call(callee, args) if matches!(callee.helper_name(), Some(\"$slice\" | \"$part\")) => {\n            Expr::call(Expr::helper(\"$copyOf\"), args.clone())\n        }\n",
    replace: "",
    tests: ["test/lowering-collections.test.ts", "-t", "a part of a slice is"],
  },
  {
    name: "clone-items-made-copied",
    breaks: "an owned copy of an array made here, a `map`'s, copies it again",
    file: "src/lower/iterators.rs",
    find: "    if made_array(&items) {\n        return items;\n    }\n    match &items.kind {\n",
    replace: "    match &items.kind {\n",
    tests: ["test/corpus.test.ts", "-t", "^(slice_to_owned|clone_from).rs$"],
    snapshots: true,
  },
  {
    name: "lowerless-iter-not-fresh",
    breaks: "`$iter(items, true)`, of what std hints no lower bound of, isn't read as an array's `$iter`, so what steps it wraps it again, `$iter($iter(items, true), true)`, and a `for` loop steps it, not `items` (#1215)",
    file: "src/lower/iterators.rs",
    find: "            [items, lowerless] if lowerless.kind == js::ExprKind::Bool(true) => Some(items),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^capacity_of_what_std_hints_less.rs$"],
    snapshots: true,
  },
  {
    name: "given-as-it-is-blind-to-what-impl-trait-hides",
    breaks:
      "a type parameter generic code gives back behind an `impl Iterator`, `fn hidden<I>(it: I) -> impl Iterator { it }`, is a JS iterator its caller makes, where the caller reveals the hidden type as the value it gave and wraps it again: the crate's own iterator yields nothing (#1230)",
    file: "src/lower/iterators.rs",
    find: "            )) if def_id.krate == id.krate && !seen.contains(&def_id) => {\n",
    replace: "            )) if false && def_id.krate == id.krate && !seen.contains(&def_id) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^own_iterator_given_on_behind_impl_trait.rs$"],
  },
  {
    name: "binding-iterator-unmarked",
    breaks: "a `dyn Iterator` a binding made, a `Set`'s `values()`, isn't marked JS's own, so its collect asks the hint its contract says is `(0, None)`, `Array.from($roomForHint(set.values(), 4, false))` where it was `Array.from(set)` (#1359)",
    file: "src/lower/iterators.rs",
    find: "    if e.facts.has(js::Known::BindingIterator) {\n        return true;\n    }\n",
    replace: "",
    tests: ["test/js-collections.test.ts", "-t", "own iteration collected"],
  },
  {
    name: "binding-iterator-mark-not-held",
    breaks: "a variable holding a binding's iterator loses its mark, so `let values = map.values(); values.collect()` asks its hint, `Array.from($roomForHint(values, 4, false))`, where React's ReactFlightWebpackPlugin writes `Array.from(chunkModules)`",
    file: "src/lower/iterators.rs",
    find: "    [js::Known::InPlace, js::Known::BindingIterator]\n",
    replace: "    [js::Known::InPlace]\n",
    tests: ["test/js-collections.test.ts", "-t", "own iteration collected"],
  },
  {
    name: "impl-associated-type-holding-param-unseen",
    breaks:
      "an impl whose associated type's value holds its type parameter, `type Output = Take<Skip<I>>` of itertools' `IteratorIndex`, is given it as a JS iterator, which its caller steps as std's struct of its own value: `next_back()` is `NaN` (#1412)",
    file: "src/lower/iterators.rs",
    find: "                .any(|item| has(tcx.type_of(item.def_id).instantiate_identity().skip_normalization()))\n",
    replace: "                .any(|item| has(tcx.type_of(item.def_id).instantiate_identity().skip_normalization()) && false)\n",
    tests: ["test/corpus.test.ts", "-t", "^index_gives_an_adapter_of_its_iterator.rs$"],
  },
  {
    name: "into-iter-named-by-another-bound-unheld",
    breaks:
      "an `IntoIterator`'s `IntoIter` another bound names, `R: IteratorIndex<I::IntoIter>` of itertools' `get`, is made a JS iterator, though that bound's impl holds it as it is: `next_back()` is `NaN` (#1412)",
    file: "src/lower/iterators.rs",
    find: "    read || named || held\n",
    replace: "    read || held\n",
    tests: ["test/corpus.test.ts", "-t", "^index_gives_an_adapter_of_its_iterator.rs$"],
  },
  {
    name: "hash-union-not-an-iterator",
    breaks: "a hash set's `union`, a JS iterator, is wrapped again where one's asked, `Iterator.from($hashSetUnion(a, b))` (#1219)",
    file: "src/lower/iterators.rs",
    find: "    \"$hashSetUnion\",\n",
    replace: "    \"$hashSetUnion-unlisted\",\n",
    tests: ["test/corpus.test.ts", "-t", "^map_set_methods.rs$"],
    snapshots: true,
  },
  {
    name: "iter-of-an-iter-kept",
    breaks: "an array's iterator made one again where what held one was read in place is `$iter($iter([1, 2]))`, not the one (#1152)",
    file: "src/lower/iterators.rs",
    find: "    while let js::ExprKind::Call(callee, args) = &mut e.kind\n        && callee.helper_name() == Some(\"$iter\")\n",
    replace: "    while let js::ExprKind::Call(callee, args) = &mut e.kind\n        && callee.helper_name() == Some(\"$iter-kept\")\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_chains_cloned_by_their_instances.rs$"],
    snapshots: true,
  },
  {
    name: "js-adapter-taken-for-compiled-struct",
    breaks: "a map's source, JS's own `.map(..)` with no mark of its own, is taken for std's compiled struct, so a map of a map given for its `len()` asks one it hasn't and throws (#1491)",
    file: "src/lower/iterators.rs",
    find: "    e.facts.has(js::Known::InPlace) || is_js_adapter(e)\n",
    replace: "    e.facts.has(js::Known::InPlace)\n",
    tests: ["test/corpus.test.ts", "-t", "^map_of_a_map_given_for_its_len.rs$"],
  },
  {
    name: "box-param-held-as-it-is",
    breaks: "generic code's `Box<I>` is taken for a value holding `I`, given as it is with `I`'s dictionary, where it's given as a JS iterator, the box of it, stepped as std's `Box<I>` steps it: right, as #1611's box dictionary steps it, and read less plainly (#1601)",
    file: "src/lower/iterators.rs",
    find: "(inputs.iter()).any(|&input| !itself(crate::lower::recognition::iterator_held(input)) && has(input));\n",
    replace: "(inputs.iter()).any(|&input| !itself(input.peel_refs()) && has(input));\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_iterator_steps_as_std_box_does.rs$"],
    snapshots: true,
  },
];
