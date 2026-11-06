// Mutations of src/lower/drops/types.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "thir-types-in-trait-terms",
    breaks:
      "a type a copied default's THIR names is asked what it drops in its trait's terms, `<Self as Iterator>::Item`, where its impl's drops are given, and `Vec::from_iter(self)` of itertools' `sorted_unstable` is refused (#828)",
    file: "src/lower/drops/types.rs",
    find: "        match self.thir_args {\n",
    replace: "        match None::<ty::GenericArgsRef<'tcx>> {\n",
    tests: ["test/corpus.test.ts", "-t", "^copied_default_asks_its_drops_in_the_impls_terms.rs$"],
  },
  {
    name: "pattern-types-in-trait-terms",
    breaks:
      "what a library's copied default's pattern moves is asked what it drops in its trait's terms, so its `[_, b]` of its `Self::Item`s is taken to move nothing that drops, and isn't refused as a generic function's is (#828)",
    file: "src/lower/drops/types.rs",
    find: "                    any |= self.has_drops(self.thir_ty(ty));\n",
    replace: "                    any |= self.has_drops(ty);\n",
    tests: ["test/diagnostics.test.ts", "-t", "copied default moving part of its items"],
  },
  {
    name: "uncounted-weak-drops",
    breaks:
      "an uncounted `Weak`'s drop is `$weakDrop` of the object itself, or of `null`, which throws (ADR 0843)",
    file: "src/lower/drops/types.rs",
    find: "            ty::Adt(..) if weak_pointee(tcx, ty).is_some() => Drops::Nothing,\n",
    replace:
      "            ty::Adt(..) if weak_pointee(tcx, ty).is_some() => Drops::Runs,\n",
    tests: ["test/corpus.test.ts", "-t", "^weak_cycle_set_after.rs$"],
    snapshots: true,
  },
  {
    name: "drops-walk-uncached",
    breaks:
      "what a type drops is found again for each path to it, which takes exponential time",
    file: "src/lower/drops/types.rs",
    find: "            self.state.cache.borrow_mut().insert(ty, found);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "nested_generic_types"],
  },
  {
    name: "drops-itself-revisits",
    breaks:
      "whether a type is inside itself walks each path to what it holds, which takes exponential time",
    file: "src/lower/drops/types.rs",
    find: "        if let Some(visited) = &mut walk.visited\n            && !visited.insert(ty)\n",
    replace:
      "        if let Some(visited) = &mut walk.visited\n            && !visited.insert(ty)\n            && false\n",
    tests: ["test/corpus.test.ts", "-t", "nested_generic_types"],
  },
  {
    name: "drops-itself-cached",
    breaks:
      "whether a type is inside itself is read from answers cached where another was outermost, which say nothing of what they reach",
    file: "src/lower/drops/types.rs",
    find: "            visited: Some(IndexSet::default()),\n",
    replace: "            visited: None,\n",
    tests: ["test/corpus.test.ts", "-t", "^trie_of_counted_rcs.rs$"],
  },
  {
    name: "walk-reach-lost",
    breaks:
      "what a type's earlier parts reached is lost to a later part's walk, a set's, so a type around it is cached as dropping nothing and a destructor in it never runs (#304)",
    file: "src/lower/drops/types.rs",
    find: "        walk.reached = walk.reached.min(outer);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^set_field_drops_in_a_cycle.rs$"],
  },
  {
    name: "dyn-drops-nothing",
    breaks:
      "a `dyn` of the crate's trait is taken to drop nothing, and what it holds is never dropped",
    file: "src/lower/drops/types.rs",
    find: "                    true => self.implementors_drop(predicates, walk),\n",
    replace: "                    true => Drops::Nothing,\n",
    tests: ["test/corpus.test.ts", "-t", "dyn_drops"],
  },
  {
    name: "library-drop-skipped",
    breaks:
      "a library's type's destructor isn't run by a crate that drops a value of it",
    file: "src/lower/drops/types.rs",
    find: "        drop.is_local() || foreign.item(drop).is_some() || foreign.is_instance(drop) || foreign.compiles(drop)\n",
    replace: "        drop.is_local() || foreign.is_instance(drop)\n",
    tests: ["test/crates.test.ts", "-t", "two crates: drop"],
  },
  {
    name: "closure-part-taken",
    breaks:
      "a closure that takes part of a value with a destructor is taken as holding it whole",
    file: "src/lower/drops/types.rs",
    find: "                    Drops::Runs if self.takes_whole(def_id) => Drops::Runs,",
    replace:
      "                    Drops::Runs if true || self.takes_whole(def_id) => Drops::Runs,",
    tests: ["test/traits.test.ts", "-t", "closure holding part"],
  },
  {
    name: "channel-ends-drop-nothing",
    breaks:
      "a channel's ends have no destructor, so a sender dropped still counts",
    file: "src/lower/drops/types.rs",
    find: "            ty::Adt(_, args) if self.recognition.channel_end(ty).is_some() => {\n                match self.drops_in(args.type_at(0), walk) {\n                    Drops::Unsupported(t, what) => Drops::Unsupported(t, what),\n                    _ => Drops::Runs,\n",
    replace:
      "            ty::Adt(_, args) if self.recognition.channel_end(ty).is_some() => {\n                match self.drops_in(args.type_at(0), walk) {\n                    Drops::Unsupported(t, what) => Drops::Unsupported(t, what),\n                    _ => Drops::Nothing,\n",
    tests: ["test/corpus.test.ts", "-t", "channels"],
  },
  {
    name: "params-drop-as-own",
    breaks:
      "what a generic iterator drops of its type parameters is taken as its own, so a generic iterator consumed, a `Peekable` of one, isn't dropped after by the drops it's given",
    file: "src/lower/drops/types.rs",
    find: "            ty::Param(_) if walk.params_none => Drops::Nothing,\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(extreme_drops|peekable_given_back_drops).rs$",
    ],
  },
  {
    name: "weak-drops-nothing",
    breaks: "dropping a `Weak` doesn't count it gone",
    file: "src/lower/drops/types.rs",
    find: "                Drops::Runs\n            }\n            ty::Adt(..) if weak_pointee(tcx, ty).is_some() => Drops::Nothing,\n",
    replace:
      "                Drops::Nothing\n            }\n            ty::Adt(..) if weak_pointee(tcx, ty).is_some() => Drops::Nothing,\n",
    tests: ["test/corpus.test.ts", "-t", "rc_counted_uses"],
  },
  {
    name: "refcell-drops-refused",
    breaks: "a `RefCell` of a `Weak` is refused",
    file: "src/lower/drops/types.rs",
    find: "                    _ if own.is_some() || std(StdItem::LazyCell) =>",
    replace: "                    _ if own.is_some() || std(StdItem::LazyCell) || std(StdItem::RefCell) =>",
    tests: ["test/corpus.test.ts", "-t", "rc_counts"],
  },
  {
    name: "map-values-drop-nothing",
    breaks: "a map's values aren't dropped with it",
    file: "src/lower/drops/types.rs",
    find: "                    self.drops_in(args.type_at(1), walk),\n                ) {\n",
    replace: "                    Drops::Nothing,\n                ) {\n",
    tests: ["test/corpus.test.ts", "-t", "map_drops"],
  },
  {
    name: "map-key-drops-allowed",
    breaks: "a map's keys with a destructor are never dropped",
    file: "src/lower/drops/types.rs",
    find: "                    self.drops_in(args.type_at(0), walk),\n                    self.drops_in(args.type_at(1), walk),\n",
    replace:
      "                    Drops::Nothing,\n                    self.drops_in(args.type_at(1), walk),\n",
    tests: ["test/corpus.test.ts", "-t", "map_key_drops"],
  },
  {
    name: "guard-drops-nothing",
    breaks: "a guard dropped leaves its cell borrowed",
    file: "src/lower/drops/types.rs",
    find: "            ty::Adt(..) if self.recognition.is_guard(ty) => Drops::Runs,",
    replace:
      "            ty::Adt(..) if self.recognition.is_guard(ty) => Drops::Nothing,",
    tests: ["test/corpus.test.ts", "-t", "refcell_borrows"],
  },
  {
    name: "guards-counted-as-drops",
    breaks: "a guard beside another is taken as dropping something that asks",
    file: "src/lower/drops/types.rs",
    find: "            ty::Adt(..) if self.recognition.is_guard(ty) && (walk.guards_none || self.uncounted_guard(ty)) => {\n",
    replace:
      "            ty::Adt(..) if self.recognition.is_guard(ty) && self.uncounted_guard(ty) => {\n",
    tests: [
      "test/lowering-cells.test.ts",
      "-t",
      "RefCell only borrowed for a moment is its value",
    ],
    snapshots: true,
  },
  {
    name: "peek-mut-drops-nothing",
    breaks: "a `PeekMut` dropped never sifts its changed top down",
    file: "src/lower/drops/types.rs",
    find: "            ty::Adt(..) if self.recognition.peek_mut_of(ty).is_some() => Drops::Runs,",
    replace:
      "            ty::Adt(..) if self.recognition.peek_mut_of(ty).is_some() => Drops::Nothing,",
    tests: ["test/corpus.test.ts", "-t", "heap_peek_mut"],
  },
  {
    name: "owned-iter-drops-unsupported",
    breaks:
      "a `vec::IntoIter` of what has a destructor is refused, as a std type holding one",
    file: "src/lower/drops/types.rs",
    find: "            ty::Adt(..) if let Some(item) = self.recognition.owned_items(ty) => self.drops_in(item, walk),\n",
    replace:
      "            ty::Adt(..) if let Some(item) = self.recognition.owned_items(ty).filter(|_| false) => self.drops_in(item, walk),\n",
    tests: ["test/corpus.test.ts", "-t", "loop_drops"],
  },
  {
    name: "given-item-drop-unknown",
    breaks:
      "a std trait's associated type whose drop the function is given is refused",
    file: "src/lower/drops/types.rs",
    find: "            ty::Alias(_, alias) if self.slot_of(*alias).is_some() => Drops::Runs,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^std_projection_drops.rs$"],
  },
  {
    name: "dyn-drop-kept-of-rust-traits-only",
    breaks:
      "a `dyn` keeps its value's drop only of a trait rust-js compiled, so a `Box<dyn Any>` or a `Box<dyn Send>` of a value with a destructor drops nothing",
    file: "src/lower/drops/types.rs",
    find: "            ty::Dynamic(predicates, ..) if self.recognition.is_pair_dyn(predicates) => {\n",
    replace:
      "            ty::Dynamic(predicates, ..)\n                if predicates.principal_def_id().is_some_and(|id| self.recognition.is_rust_trait(id)) =>\n            {\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(dyn_std_destructors|dyn_auto_traits).rs$",
    ],
  },
  {
    name: "unsize-array-as-dyn",
    breaks:
      "an array unsized to a slice in a trait's default method is taken for a `dyn`, and rejected",
    file: "src/lower/drops/types.rs",
    find: "        let ty::Dynamic(predicates, ..) = to.kind() else {\n            return false;\n",
    replace:
      "        let ty::Dynamic(predicates, ..) = to.kind() else {\n            return to.is_slice() && self.drops(from) != Drops::Nothing;\n",
    tests: ["test/corpus.test.ts", "-t", "default_method_drops"],
  },
  {
    name: "dyn-reference-refused",
    breaks:
      "a `&dyn` of a value with a destructor in a trait's default method, which owns nothing to drop, is rejected",
    file: "src/lower/drops/types.rs",
    find: "        !target.is_ref()\n            && match self.drops(from) {\n",
    replace: "        match self.drops(from) {\n",
    tests: ["test/corpus.test.ts", "-t", "default_method_drops"],
  },
  {
    name: "dyn-implementors-unread",
    breaks:
      "a `dyn` of the crate's trait drops something wherever a type may, though none of its implementors drops anything, so its `Rc`, and its owner's, are counted, `{ value, strong, weak }`",
    file: "src/lower/drops/types.rs",
    find: "        !implementor.has_non_region_param() && !drops(implementor)\n",
    replace: "        false\n",
    tests: [
      "test/lowering.test.ts",
      "-t",
      "an Rc of a dyn none of whose implementors drops",
    ],
  },
  {
    name: "held-function-unsupported",
    breaks:
      "an adapter that holds a function with a destructor is a std type holding one, refused",
    file: "src/lower/drops/types.rs",
    find: "            ty::Adt(_, args) if let Some(function) = self.recognition.held_function(ty) => {\n",
    replace:
      "            ty::Adt(_, args) if let Some(function) = self.recognition.held_function(ty).filter(|_| false) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^held_function_drops.rs$"],
  },
  {
    name: "item-slot-ungiven-hooked",
    breaks:
      "`counted(0..3)`, whose items no caller gives a drop, is given a hook for one, `IIterator.$dropItem?.(item)`",
    file: "src/lower/drops/types.rs",
    find: "            ty::Alias(_, alias) if self.is_slot(*alias) => Drops::Nothing,\n",
    replace: "",
    tests: [
      "test/lowering.test.ts",
      "-t",
      "a generic count of items no caller gives a drop for drops none",
    ],
  },
  {
    name: "operator-output-drop-unseen",
    breaks:
      "a generic `a + b`'s `Output` with a destructor isn't dropped by its dictionary's `$dropOutput`, and is refused",
    file: "src/lower/drops/types.rs",
    find: "        (!tcx.is_impl_trait_in_trait(def_id)).then(|| (alias.trait_ref(tcx), def_id))\n",
    replace: "        None\n",
    tests: ["test/corpus.test.ts", "-t", "^operator_output_drops.rs$"],
  },
  {
    name: "extract-if-drops-items",
    breaks:
      "an `extract_if()`'s iterator is taken to drop what it hasn't reached, refused",
    file: "src/lower/drops/types.rs",
    find: "            ty::Adt(..) if let Some(test) = self.recognition.extract_if_test(ty) => self.drops_in(test, walk),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^extract_if_drops.rs$"],
  },
  {
    name: "cell-drops-refused",
    breaks: "a `Cell` or a `OnceCell` of a value with a destructor is refused",
    file: "src/lower/drops/types.rs",
    find: "                    _ if own.is_some() || std(StdItem::LazyCell) =>",
    replace:
      "                    _ if own.is_some() || std(StdItem::LazyCell) || std(StdItem::Cell) || std(StdItem::OnceCell) =>",
    tests: ["test/corpus.test.ts", "-t", "^cell_drops.rs$"],
  },
  {
    name: "fn-dyn-drops-nothing",
    breaks:
      "a `dyn Fn` drops nothing, though a function holding a value with a destructor is made one",
    file: "src/lower/drops/types.rs",
    find: "                match self.closure_facts.fn_dyns_drop {\n                    true => Drops::Runs,\n",
    replace:
      "                match self.closure_facts.fn_dyns_drop {\n                    true => Drops::Nothing,\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_drops.rs$"],
  },
  {
    name: "foreign-closure-drop-refused",
    breaks:
      "a closure another crate gives back, holding a value with a destructor, is refused where it's dropped",
    file: "src/lower/drops/types.rs",
    find: "            || !closure.is_local() && crate::lower::recognition::given_back(self.recognition.tcx, closure)\n",
    replace:
      "            || false && crate::lower::recognition::given_back(self.recognition.tcx, closure)\n",
    tests: ["test/crates.test.ts", "-t", "three crates: rc_generic"],
  },
  {
    name: "any-dyn-drops-by-the-crate",
    breaks:
      "every `dyn Any` of a crate with destructors drops, a caught panic's payload too, so `catch_unwind(..).unwrap_or_default()` is `$catchUnwind` and a `kept`",
    file: "src/lower/drops/types.rs",
    find: "            return match self.closure_facts.any_dyns_drop {\n",
    replace: "            return match true {\n",
    tests: [
      "test/lowering-unwinds.test.ts",
      "-t",
      "destructors leave catch_unwind",
    ],
  },
  {
    name: "async-fn-future-holds-arguments",
    breaks:
      "an `async fn`'s future is taken to hold its arguments, which its body drops too",
    file: "src/lower/drops/types.rs",
    find: "                        || crate::lower::recognition::is_async_block(self.recognition.tcx, def_id)) =>\n",
    replace: "                        || true) =>\n",
    tests: [
      "test/lowering-functions.test.ts",
      "-t",
      "an async block or closure drops what it took",
    ],
  },
  {
    name: "quiet-guard-dropped",
    breaks:
      "a guard of a `RefCell` no borrow of which can meet another is released where it's dropped, `$unborrow(types)`, though it was never counted (ADR 0863)",
    file: "src/lower/drops/types.rs",
    find: "            ty::Adt(..) if self.recognition.is_guard(ty) && (walk.guards_none || self.uncounted_guard(ty)) => {\n",
    replace:
      "            ty::Adt(..) if self.recognition.is_guard(ty) && walk.guards_none => {\n",
    // refcell_quiet_borrows' guards are each a local, which the drop of
    // one skips itself (mir/drops.rs), so its JS is the same with it; a
    // guard of a field read through, `fiber.r#return.borrow()`, is held as a
    // `const` with it (the Nightly of 3c71ee8f9). Since, refcell_field_places'
    // JS is the same with it too, and of the whole corpus only cell_const's
    // changes (the Nightly of 85f123290, #725).
    tests: ["test/corpus.test.ts", "-t", "^cell_const.rs$"],
    snapshots: true,
  },
  {
    name: "projection-drop-unnormalized",
    breaks: "an associated type its bounds say is another is never read as that one, so `Iterator::count`'s closure inlined in std's `FlattenCompat::count` is refused its item's drop, and std's bodies don't compile (#736)",
    file: "src/lower/drops/types.rs",
    find: "                if let Some(normal) = self.normalized(ty)\n                    && normal != ty =>\n",
    replace: "                if let Some(normal) = self.normalized(ty)\n                    && normal != ty\n                    && false =>\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std"],
  },
  {
    name: "impossible-alias-unsupported",
    breaks:
      "an associated type of a type its trait isn't implemented for, an impl's slot of a default its arguments can't meet, is refused as an unknown one, so `(0..3u8).counted()` doesn't compile (#798)",
    file: "src/lower/drops/types.rs",
    find: "    if !ty.has_param() {\n        return tcx.instantiate_and_check_impossible_clauses((*def_id, alias.args));\n",
    replace: "    if !ty.has_param() {\n        return false;\n",
    tests: ["test/corpus.test.ts", "-t", "^default_where_clause_item_drops.rs$"],
  },
  {
    name: "unmet-generic-alias-unsupported",
    breaks:
      "generic code calling a blanket impl's default is asked the drop of an associated type another default's where clause names, which it can't meet there, `<<I::IntoIter as Iterator>::Item as IntoIterator>::Item`, and is refused, as itertools' `intersperse` was (#828)",
    file: "src/lower/drops/types.rs",
    find: "            .try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(ty))\n            .is_err()\n",
    replace: "            .try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(ty))\n            .is_err()\n            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^blanket_default_beside_an_unmet_clause.rs$"],
  },
  {
    name: "binding-at-whole-unfollowed",
    breaks:
      "`elt @ Some(_)` of a temporary with a destructor, whose subpattern moves nothing, is refused as moving part of it, as itertools' `collect_tuple` was (#828)",
    file: "src/lower/drops/types.rs",
    find: "            } if !moves(sub) => {\n",
    replace: "            } if false && !moves(sub) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^default_matches_its_tuple_whole.rs$"],
  },
  {
    name: "no-type-of-unbounded-params",
    breaks:
      "an associated type of another item's type parameter, a dictionary's method's own `I`, read where its impl's bounds hold, is taken for no type there, so `from_iter::<I>`'s in-place buffer isn't given and a panic drops in another order (#794, #828)",
    file: "src/lower/drops/types.rs",
    find: "    params_bounded\n        && tcx\n",
    replace: "    tcx\n",
    tests: ["test/libraries.test.ts", "-t", "a std trait's generic method's dictionary carries the same facts in each library"],
  },
];
