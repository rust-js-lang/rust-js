// Mutations of crates/rust-js-lower/src/iterators.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "into-iter-named-not-held",
    breaks:
      "an `IntoIterator` type parameter `impl Is<I>` names is made a JS iterator by `Iterator.from` where its caller gave its own value, a crate's iterator struct (#966)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "        && bound_as_iterator(tcx, id, param)\n        && named_by_another_bound(tcx, id, param)\n",
    replace: "        && false\n",
    tests: ["test/corpus.test.ts", "-t", "^into_iterator_named_by_another_bound.rs$"],
  },
  {
    name: "naming-parameter-of-into-iter-boxed",
    breaks:
      "`impl Is<I>` of an `IntoIterator` held as it is isn't given as it is, so a `Vec`'s iterator of items whose drop may panic given for it is wrapped in its slice's steps, `$slicing(it, drop)`, which `I`'s dictionary steps already (#966, #1109)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "                        && into_iter_held_as_it_is(tcx, id, Ty::new_param(tcx, p.index, p.name)) =>\n",
    replace: "                        && false =>\n",
    tests: ["test/corpus.test.ts", "-t", "^into_iterator_named_by_another_bound.rs$"],
    snapshots: true,
  },
  {
    name: "reversal-of-range-chain-as-items",
    breaks:
      "a map of a range stepped from its end, made in place, is read as its source's map, which steps the range forward: `Some(2)` where Rust gives `Some(9)` (ADR 0987)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "            _ if is_stepped_items(e) => true,\n",
    replace:
      "            _ if is_stepped_items(e) || steps(e).is_some() => true,\n",
    tests: ["test/corpus.test.ts", "-t", "^(rev_made_knowing_its_back|map_of_a_reversed_range_given_for_its_length).rs$"],
  },
  {
    name: "made-array-helper-lazy",
    breaks:
      "`v.windows(2).map(f).collect()` from MIR stays `Array.from(Iterator.from($windows(v, 2)).map(f))`",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "        js::ExprKind::Call(callee, _) if matches!(callee.kind, js::ExprKind::Var(_)) => fresh(e),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(calculator|map_set_methods).rs$"],
    snapshots: true,
  },
  {
    name: "mir-object-keys-iterated",
    breaks:
      "`Object.keys(o)`, an array, is taken as a JS iterator and stepped as one",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "            js::ExprKind::Var(v) if v.is(\"Object\") => false,\n",
    replace: "",
    tests: ["test/lowering-loops.test.ts", "-t", "a for over Object.keys"],
  },
  {
    name: "mir-adapter-not-js-iterator",
    breaks:
      "an adapter of a JS iterator, `.map(f)` of a range's, is made one again, `Iterator.from(..)`",
    file: "crates/rust-js-lower/src/iterators.rs",
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
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "        has(tcx.type_of(imp).instantiate_identity().skip_normalization())\n            || (tcx.associated_items(imp)",
    replace: "        (has(tcx.type_of(imp).instantiate_identity().skip_normalization()) && false)\n            || (tcx.associated_items(imp)",
    tests: ["test/corpus.test.ts", "-t", "^iterator_adapters.rs$"],
  },
  {
    name: "given-as-it-is-not-given-back",
    breaks:
      "`doubled(0..4)`, whose result holds its `I`, converts the range to a JS iterator its caller steps as a range",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "            has(sig.output()) || meet || inside\n",
    replace: "            false || meet || inside\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_adapters.rs$"],
  },
  {
    name: "naming-parameter-not-as-it-is",
    breaks:
      "`impl Is<I>` of an iterator given as it is isn't given as it is, so a `Vec`'s iterator of items whose drop may panic given for it is wrapped in its slice's steps, `$slicing(it, drop)`, which `I`'s dictionary steps already (#966, #1109)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "        return names_given_as_it_is(tcx, (id, args), param);\n",
    replace: "        return false;\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_named_by_another_bound.rs$"],
    snapshots: true,
  },
  {
    name: "given-as-it-is-not-in-input",
    breaks:
      "`second(Wrap(7..10))`'s range, in what's given, is stepped as a JS iterator",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "            let inside = (inputs.iter()).any(|&input| !itself(crate::recognition::iterator_held(input)) && has(input));\n",
    replace: "            let inside = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_adapters.rs$"],
  },
  {
    name: "given-as-it-is-projection",
    breaks:
      "`all<I>(i) -> Vec<I::Item>`, which gives back only its items, takes its `I` as it is, with a dictionary",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "            ty::Alias(..) => walk.skip_current_subtree(),\n            // Not of `id`'s own crate, std's function's own adapter, `Map<I,\n",
    replace: "            // Not of `id`'s own crate, std's function's own adapter, `Map<I,\n",
    tests: ["test/corpus.test.ts", "-t", "^associated_types.rs$"],
    snapshots: true,
  },
  {
    name: "cycle-wrapped",
    breaks:
      "a `$cycle`'s generator is wrapped in `Iterator.from(..)`, though it's a JS iterator already",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: '    "$cycle",\n',
    replace: '    "$cycled",\n',
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^iter_cycle.rs$"],
  },
  {
    name: "lazy-chain-not-iterator",
    breaks:
      "`$lazyChain(a, b)`, a generator, is taken for what isn't a JS iterator, and wrapped in `Iterator.from`",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: '    "$lazyChain",\n',
    replace: '    "$lazyChained",\n',
    tests: ["test/corpus.test.ts", "-t", "^adapter_drops.rs$"],
    snapshots: true,
  },
  {
    name: "iterator-read-as-js",
    breaks:
      "the crate's iterator given to `I: Iterator + Clone` is its JS iterator, which its `Clone` and `Debug` read as its value",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "    held || signature || read\n",
    replace: "    held || signature\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_read_as_itself.rs$"],
  },
  {
    name: "extension-bound-read",
    breaks:
      "a `DoubleEndedIterator` bound is taken for one that reads its iterator as its own type, which is then given as it is",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "        && !super::recognition::is_iterator_extension(tcx, tr)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^associated_double_ended.rs$"],
    snapshots: true,
  },
  {
    name: "extension-bound-not-given-as-is",
    breaks:
      "an `I: DoubleEndedIterator` a type holds is a JS iterator in its impl, which its caller's range isn't",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "        .any(|&tr| tr == iterator || super::recognition::implies_iterator(tcx, tr))\n",
    replace: "        .any(|&tr| tr == iterator)\n",
    tests: ["test/corpus.test.ts", "-t", "^extension_bound_iterators.rs$"],
  },
  {
    name: "closing-not-an-iterator",
    breaks:
      "`$closing` is taken for what isn't a JS iterator, written inside `Iterator.from`, and its chain not made eager",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: '    "$closing",\n',
    replace: '    "$peekable",\n',
    tests: ["test/corpus.test.ts", "-t", "^adapter_drops.rs$"],
    snapshots: true,
  },
  {
    name: "std-provided-self-given-by-trait",
    breaks:
      "a compiled std provided method's `Self`, `peekable`'s, isn't given as it is, so `@rust-js/std`'s `iterator_peekable` takes no dictionary for it, and a caller's generic iterator steps nothing",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "    ) && (tcx.trait_of_assoc(id).is_none()\n        || match args {\n",
    replace: "    ) && (tcx.trait_of_assoc(id).is_none()\n        || false && match args {\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "into-iter-never-held",
    breaks:
      "no `IntoIterator`'s `IntoIter` is held as it is, so a range is given as a JS iterator and stepped as a range",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "    read || named || held\n",
    replace: "    false\n",
    tests: ["test/corpus.test.ts", "-t", "^into_iter_held_as_it_is.rs$"],
  },
  {
    name: "into-iter-param-made-js-iterator",
    breaks:
      "a type parameter whose `IntoIter` is held as it is is made a JS iterator by its caller",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "    if into_iter_held_as_it_is(tcx, id, Ty::new_param(tcx, param.index, param.name)) {\n        return true;\n    }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^into_iter_held_as_it_is.rs$"],
  },
  {
    name: "into-iter-held-in-struct-a-js-iterator",
    breaks:
      "an `IntoIterator` whose `IntoIter` a function holds in a struct of the crate's, `Pair<I::IntoIter>`, is given a JS iterator, which a compiled `Fuse` then holds where its callers read the struct",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "    read || named || held\n",
    replace: "    read || named\n",
    tests: ["test/corpus.test.ts", "-t", "^fused_iterators.rs$"],
  },
  {
    name: "sliced-not-made",
    breaks:
      "a slice's range in bounds, `items.slice(0, 3)`, isn't an array made here, so its `to_vec()` copies it again, `.slice(0, 3).slice()` (ADR 0806)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "        _ if e.facts.has(js::Known::Sliced) => true,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "slice_ranges_known"],
    snapshots: true,
  },
  {
    name: "alias-into-iter-not-held",
    breaks:
      "generic code given `I::Item: IntoIterator` iterates each item as a JS iterator, which an `Option`'s `T | undefined` isn't (#278)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "    if matches!(*into.kind(), ty::Alias(_, ty::AliasTy { kind: ty::Projection { def_id }, .. }) if def_id != into_iter)\n    {\n",
    replace:
      "    if false && matches!(*into.kind(), ty::Alias(_, ty::AliasTy { kind: ty::Projection { def_id }, .. }) if def_id != into_iter)\n    {\n",
    tests: ["test/corpus.test.ts", "-t", "^into_iter_of_items.rs$"],
  },
  {
    name: "held-into-iter-param-not-given",
    breaks:
      "`U` of `I::Item: IntoIterator<IntoIter = U>` is a JS iterator, so `Flatten`'s `next` gives `FlattenCompat` a generic `{ next }` of what's held as it is",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "    if is_held_into_iter(tcx, id, param) {\n",
    replace: "    if false && is_held_into_iter(tcx, id, param) {\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "stepped-slices-length-kept",
    breaks:
      "a slice's iterator asked its length too, `it.len()`, is made the array's own `values()`, which has none, a TypeError (rd270)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "        if declared != 1 || steps == 0 || js::mentions_of(body, &name) != steps {\n",
    replace: "        if declared != 1 || steps == 0 {\n",
    tests: ["test/lowering-iterators.test.ts", "-t", "only stepped"],
  },
  {
    name: "known-iterator-made-again",
    breaks:
      "a variable its type says is a JS iterator, a held `take_while`'s, is made one again by `Iterator.from`, whose `[Symbol.iterator]` makes it close as it's done, dropping what's left before the statement ends (generated program 12454000)",
    file: "crates/rust-js-lower/src/iterators.rs",
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
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "&& given.count() > 1;",
    replace: "&& given.count() > 2;",
    tests: ["test/corpus.test.ts", "-t", "^iterator_values_meet.rs$"],
  },
  {
    name: "extension-step-held",
    breaks:
      "`v.iter().map(f).rfold(..)` takes the `map()` as std's struct, which `DoubleEndedIterator`'s methods steps where it's made",
    file: "crates/rust-js-lower/src/iterators.rs",
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
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "        e.facts.has(js::Known::Iterator) && method == \"map\" && f.reads_only_vars()\n",
    replace: "        method == \"map\" && f.reads_only_vars()\n",
    tests: ["test/corpus.test.ts", "-t", "^unread_map_of_a_held_iterator.rs$"],
  },
  {
    name: "into-iter-hidden-by-a-library-unseen",
    breaks:
      "whether a function holds what an `IntoIterator` makes as it is, behind an `impl Iterator` it gives, asks of an opaque type of the crate compiling, so a library's `chained`'s consumer gives no dictionary its JS takes (#1025)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "            ) if def_id.krate == id.krate && !seen.contains(&def_id) => {\n",
    replace: "            ) if def_id.is_local() && !seen.contains(&def_id) => {\n",
    tests: ["test/crates-pairs-1.test.ts", "-t", "two crates: library_into_iter_held_as_it_is"],
  },
  {
    name: "owned-range-copied-twice",
    breaks: "an owned copy of a range, `v[..n].to_vec()`, is the part copied again, `$slice(v, 0, n).slice()`, two copies where one is",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "        js::ExprKind::Call(callee, args) if matches!(callee.helper_name(), Some(\"$slice\" | \"$part\")) => {\n            Expr::call(Expr::helper(\"$copyOf\"), args.clone())\n        }\n",
    replace: "",
    tests: ["test/lowering-collections.test.ts", "-t", "a part of a slice is"],
  },
  {
    name: "clone-items-made-copied",
    breaks: "an owned copy of an array made here, a `map`'s, copies it again",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "    if made_array(&items) {\n        return items;\n    }\n    match &items.kind {\n",
    replace: "    match &items.kind {\n",
    tests: ["test/corpus.test.ts", "-t", "^(slice_to_owned|clone_from).rs$"],
    snapshots: true,
  },
  {
    name: "lowerless-iter-not-fresh",
    breaks: "`$iter(items, true)`, of what std hints no lower bound of, isn't read as an array's `$iter`, so what steps it wraps it again, `$iter($iter(items, true), true)`, and a `for` loop steps it, not `items` (#1215)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "            [items, lowerless] if lowerless.kind == js::ExprKind::Bool(true) => Some(items),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^capacity_of_what_std_hints_less.rs$"],
    snapshots: true,
  },
  {
    name: "given-as-it-is-blind-to-what-impl-trait-hides",
    breaks:
      "a type parameter generic code gives back behind an `impl Iterator`, `fn hidden<I>(it: I) -> impl Iterator { it }`, is a JS iterator its caller makes, where the caller reveals the hidden type as the value it gave and wraps it again: the crate's own iterator yields nothing (#1230)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "            ) if def_id.krate == id.krate && !seen.contains(&def_id) => {\n",
    replace: "            ) if false && def_id.krate == id.krate && !seen.contains(&def_id) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^own_iterator_given_on_behind_impl_trait.rs$"],
  },
  {
    name: "binding-iterator-unmarked",
    breaks: "a `dyn Iterator` a binding made, a `Set`'s `values()`, isn't marked JS's own, so its collect asks the hint its contract says is `(0, None)`, `Array.from($roomForHint(set.values(), 4, false))` where it was `Array.from(set)` (#1359)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "    if e.facts.has(js::Known::BindingIterator) {\n        return true;\n    }\n",
    replace: "",
    tests: ["test/js-collections.test.ts", "-t", "own iteration collected"],
  },
  {
    name: "binding-iterator-mark-not-held",
    breaks: "a variable holding a binding's iterator loses its mark, so `let values = map.values(); values.collect()` asks its hint, `Array.from($roomForHint(values, 4, false))`, where React's ReactFlightWebpackPlugin writes `Array.from(chunkModules)`",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "    [js::Known::InPlace, js::Known::BindingIterator]\n",
    replace: "    [js::Known::InPlace]\n",
    tests: ["test/js-collections.test.ts", "-t", "own iteration collected"],
  },
  {
    name: "impl-associated-type-holding-param-unseen",
    breaks:
      "an impl whose associated type's value holds its type parameter, `type Output = Take<Skip<I>>` of itertools' `IteratorIndex`, is given it as a JS iterator, which its caller steps as std's struct of its own value: `next_back()` is `NaN` (#1412)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "                .any(|item| has(tcx.type_of(item.def_id).instantiate_identity().skip_normalization()))\n",
    replace: "                .any(|item| has(tcx.type_of(item.def_id).instantiate_identity().skip_normalization()) && false)\n",
    tests: ["test/corpus.test.ts", "-t", "^index_gives_an_adapter_of_its_iterator.rs$"],
  },
  {
    name: "into-iter-named-by-another-bound-unheld",
    breaks:
      "an `IntoIterator`'s `IntoIter` another bound names, `R: IteratorIndex<I::IntoIter>` of itertools' `get`, is made a JS iterator, though that bound's impl holds it as it is: `next_back()` is `NaN` (#1412)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "    read || named || held\n",
    replace: "    read || held\n",
    tests: ["test/corpus.test.ts", "-t", "^index_gives_an_adapter_of_its_iterator.rs$"],
  },
  {
    name: "hash-union-not-an-iterator",
    breaks: "a hash set's `union`, a JS iterator, is wrapped again where one's asked, `Iterator.from($hashSetUnion(a, b))` (#1219)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "    \"$hashSetUnion\",\n",
    replace: "    \"$hashSetUnion-unlisted\",\n",
    tests: ["test/corpus.test.ts", "-t", "^map_set_methods.rs$"],
    snapshots: true,
  },
  {
    name: "iter-of-an-iter-kept",
    breaks: "an array's iterator made one again where what held one was read in place is `$iter($iter([1, 2]))`, not the one (#1152)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "    while (e.helper_call_of(\"$iter\").and_then(|made| made.arg(\"items\"))).is_some_and(is_stepped_items)\n",
    replace: "    while (e.helper_call_of(\"$iter\").and_then(|made| made.arg(\"items\"))).is_some_and(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_chains_cloned_by_their_instances.rs$"],
    snapshots: true,
  },
  {
    name: "js-adapter-taken-for-compiled-struct",
    breaks: "a map's source, JS's own `.map(..)` with no mark of its own, is taken for std's compiled struct, so a map of a map given for its `len()` asks one it hasn't and throws (#1491)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "    e.facts.has(js::Known::InPlace) || is_js_adapter(e)\n",
    replace: "    e.facts.has(js::Known::InPlace)\n",
    tests: ["test/corpus.test.ts", "-t", "^map_of_a_map_given_for_its_len.rs$"],
  },
  {
    name: "box-param-held-as-it-is",
    breaks: "generic code's `Box<I>` is taken for a value holding `I`, given as it is with `I`'s dictionary, where it's given as a JS iterator, the box of it, stepped as std's `Box<I>` steps it: right, as #1611's box dictionary steps it, and read less plainly (#1601)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "(inputs.iter()).any(|&input| !itself(crate::recognition::iterator_held(input)) && has(input));\n",
    replace: "(inputs.iter()).any(|&input| !itself(input.peel_refs()) && has(input));\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_iterator_steps_as_std_box_does.rs$"],
    snapshots: true,
  },
  {
    name: "hintless-form-forgotten-by-its-variable",
    breaks: "a variable that holds JS's own iterator, a map of a slice's `values()` a `let` holds, keeps no fact of it, so a `Vec` extended by a map of it is given the `true` that asks a fold it hasn't, `$extend(w, d.map(g), true)`, a flag that changes nothing, where #1803 gives it none; the capacity it guarded for #1792 is the maker's since #1795 (ADR 1170)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "    match hintless(value) {\n        true => marks.with(js::Known::IterHintless),\n",
    replace: "    match false {\n        true => marks.with(js::Known::IterHintless),\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^extend_by_an_adapter_of_a_held_js_iterator.rs$"],
  },
  {
    name: "deque-range-copied-as-a-part",
    breaks: "a deque's `range(a..b)` collected is `$sliceRange(v, a, b).slice()`, a part copied, not `$copyRange(v, a, b)`, one copy checked as `slice::range` checks it: right, but not the JS a person writes (#1937)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: '        js::ExprKind::Call(callee, args) if callee.helper_name() == Some("$sliceRange") => {\n',
    replace: '        js::ExprKind::Call(callee, args) if callee.helper_name() == Some("$sliceRange") && false => {\n',
    tests: ["test/corpus.test.ts", "-t", "^capacity_of_what_std_hints_less.rs$"],
    snapshots: true,
  },
  {
    name: "linked-source-base-unwrapped",
    breaks: "JS's own chain made again without its base wrapped in `$stepsAround`, so its first adapter is JS's `Iterator.prototype` one, which keeps no link to the source `Vec`, and the in-place capacity throws (#1941)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "            Expr::call_helper(\"$stepsAround\", vec![(**object).clone()])\n",
    replace: "            (**object).clone()\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_collect_in_generic_code_of_a_hidden_source.rs$"],
  },
  {
    name: "impl-type-holding-into-iter-unasked",
    breaks: "an impl whose type holds what its parameter's `into_iter()` makes takes no dictionary of it, so its dictionary is one module const whose method is given a JS iterable's whatever the caller's `A`, and a tuple struct's iterator is stepped as its fields (#2263)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "        rustc_hir::def::DefKind::Impl { .. } => {\n",
    replace: "        rustc_hir::def::DefKind::Impl { .. } if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^impl_dictionary_over_an_into_iterator_its_type_holds.rs$"],
  },
  {
    name: "in-place-rev-a-struct",
    breaks:
      "a `Rev` stepped where it's made is taken as std's struct, closed after it's consumed",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: " && crate::iterators::made_in_place(value))\n",
    replace: " && false)\n",
    snapshots: true,
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^own_iterator_dropped_through_adapters.rs$",
    ],
  },
  {
    name: "assoc-iterator-a-js-iterator",
    breaks:
      "a trait's associated iterator is a JS iterator in generic code, which its impl's method makes one of, so a range it gives back to concrete code reads as \`undefined..undefined\` and a \`From<I>\` bound's value steps as a JS iterator, a \`TypeError\` (#2328, #2330)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "    !super::recognition::is_std_crate(tcx, item)\n        && matches!(tcx.def_kind(item), rustc_hir::def::DefKind::AssocTy)\n",
    replace: "    false\n        && matches!(tcx.def_kind(item), rustc_hir::def::DefKind::AssocTy)\n",
    tests: ["test/associated-iterator-matrix.test.ts"],
  },
  {
    name: "assoc-iterator-impl-trait-in-trait-as-it-is",
    breaks:
      "a trait method's `-> impl DoubleEndedIterator`, rustc's associated type of it, is taken for its type's own value, so generic code's `rev()` of it is refused (#2330)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "        && !tcx.is_impl_trait_in_trait(item)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^associated_iterator_of_a_method_with_its_own_generics.rs$"],
  },
  {
    name: "into-iter-equated-by-a-bound-unread",
    breaks:
      "a `J: IntoIterator<IntoIter = I>` is given as a JS iterable, whose `$intoIter` the body holds as `I`, stepped by `I`'s own `next`: the crate's iterator of a field steps `undefined`, a `Range`'s fields read `undefined` (#2520)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "        .any(|p| p.def_id() == into_iter && p.self_ty() == into);\n",
    replace: "        .any(|p| p.def_id() == into_iter && p.self_ty() == into && false);\n",
    tests: ["test/corpus.test.ts", "-t", "^into_iter_equated_by_a_bound\\.rs$"],
  },
  {
    name: "into-iter-in-a-signature-unread",
    breaks:
      "a function that gives back what an `IntoIterator` makes, `-> T::IntoIter`, `Option<T::IntoIter>`, or takes one, takes its `T` as a JS iterator, so concrete code reads a range given back as `undefined..undefined` (#2331)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "    let holds = |ty: Ty<'tcx>| holds_readably(tcx, id, ty, &is_its_iter, &mut Vec::new());\n",
    replace: "    let holds = |ty: Ty<'tcx>| holds_readably(tcx, id, ty, &|_| false, &mut Vec::new());\n",
    tests: ["test/associated-iterator-matrix.test.ts"],
  },
  {
    name: "readable-walk-reads-into-adapters",
    breaks:
      "what std's adapter made a JS iterator holds, `Copied<I>` given or `Zip<T::IntoIter, J>` given back, is taken as read by its caller, so its `I` is given as it is, with dictionaries no step needs (#2331)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "                if super::recognition::is_iter_adapter_def(tcx, adt.did())\n",
    replace: "                if false && super::recognition::is_iter_adapter_def(tcx, adt.did())\n",
    tests: ["test/corpus.test.ts", "-t", "^uncompiled_adapter_given_to_generic_code.rs$"],
    snapshots: true,
  },
  {
    name: "impl-into-iter-a-js-iterator",
    breaks:
      "an impl's `IntoIterator` dictionary makes a JS iterator of its `IntoIter`, `$iterator(bagIntoIterator_intoIter(bag), $rangeNext)`, which generic code holding it as it is gives back to concrete code that reads a range: `undefined..undefined` (#2331)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "    if item == trait_method(tcx, std_item(tcx, StdItem::IntoIterator), \"IntoIter\") {\n        return true;\n    }\n",
    replace: "",
    tests: ["test/associated-iterator-matrix.test.ts"],
  },
  {
    name: "dictionary-into-iter-a-js-iterator",
    breaks:
      "generic code given a reference's `IntoIterator` dictionary, `for<'a> &'a C: IntoIterator`, steps what its `into_iter` makes as a JS iterator, so the crate's impl's range is no iterator, a `TypeError` (#2331)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "    if super::recognition::into_iter_by_dictionary(tcx, into_iterator, into) {\n        return true;\n    }\n",
    replace: "",
    tests: ["test/associated-iterator-matrix.test.ts"],
  },
  {
    name: "readable-walk-skips-stds-own-adapters",
    breaks:
      "std's own compile of its adapter, `@rust-js/std`'s `Map<I, F>` made of a parameter, is skipped as a JS iterator, so its `I` loses its dictionary, and the package's manifest its `stepped` facts (#2331)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "                    && adt.did().krate != id.krate =>\n",
    replace: "                    && true =>\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "fresh-iter-sliced",
    breaks:
      "what's left of a just-made `$iter(items)` is `it.items.slice(it.at, it.end)`, not `items`",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: '        js::ExprKind::Call(callee, args) if callee.helper_name() == Some("$iter") => match args.as_slice() {\n',
    replace: '        js::ExprKind::Call(callee, args) if false && callee.helper_name() == Some("$iter") => match args.as_slice() {\n',
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^iter_cycle.rs$"],
  },
  {
    name: "into-iterator-another-bound-unread",
    breaks:
      "a `T: IntoIterator + Clone` is given the JS iterator its caller makes of its value, which its `Clone` dictionary reads as the value: a range's clone reads `.start` of a `$Steps`, a fieldless enum's place `.value` of one, the crate's iterator's clone shares its steps (#2540)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "(is_its_iter(p.self_ty()) || (an_into_iterator && p.self_ty() == into && p.def_id() != into_iterator))",
    replace: "is_its_iter(p.self_ty())",
    tests: ["test/corpus.test.ts", "-t", "^into_iterator_another_bound_reads\\.rs$"],
  },
  {
    name: "copy-reads-no-value",
    breaks:
      "a `T: Iterator + Copy` is given the JS iterator its caller makes of its value, whose copy function a fieldless enum's place has none of: `TCopy.copy is not a function` (#2540)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "tcx.is_lang_item(tr, rustc_hir::attrs::lang_items::LangItem::Copy)\n        ||",
    replace: "false\n        ||",
    tests: ["test/corpus.test.ts", "-t", "^into_iterator_another_bound_reads\\.rs$"],
  },
  {
    name: "items-of-an-iterator-stepped-as-js",
    breaks:
      "an iterator's items stepped through their own bound, `S::Item: Iterator`, are stepped as JS iterators, where they come as their caller holds them: the crate's iterator's object, a range's: `it.next is not a function` (#2541)",
    file: "crates/rust-js-lower/src/iterators.rs",
    find: "            .any(|p| p.def_id() == iterator && p.skip_binder().self_ty() == ty)\n    {\n        return true;",
    replace: "            .any(|p| p.def_id() == iterator && p.skip_binder().self_ty() == ty && false)\n    {\n        return true;",
    tests: ["test/corpus.test.ts", "-t", "^items_stepped_through_their_bound\\.rs$"],
  },
];
