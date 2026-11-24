// Mutations of crates/rust-js-lower/src/analysis/type_facts/calls.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "type-facts-unsized-asked",
    breaks:
      "`size_of_val` of an unsized `T` asks its caller for a size its type hasn't",
    file: "crates/rust-js-lower/src/analysis/type_facts/calls.rs",
    find: "                of.is_sized(tcx, typing_env) || !matches!(fact, TypeFact::Size | TypeFact::Room | TypeFact::Align)\n",
    replace: "                true\n",
    tests: ["test/diagnostics.test.ts", "-t", "size_of_val of an unsized"],
  },
  {
    name: "any-bound-fact-asked",
    breaks: "a `T: Any` is given its `TypeId` beside its dictionary",
    file: "crates/rust-js-lower/src/analysis/type_facts/calls.rs",
    find: "                && !(fact == TypeFact::Id && bound_by_any(tcx, caller, param.index))",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^dyn_any.rs"],
    snapshots: true,
  },
  {
    name: "library-facts-unpassed",
    breaks:
      "a consumer's generic function that passes its `T` to a library's that asks its size doesn't ask it",
    file: "crates/rust-js-lower/src/analysis/type_facts/calls.rs",
    find: "(caller, args), (slot, fact)) {\n                        self.ask(caller, each);\n",
    replace: "(caller, args), (slot, fact)) {\n                        let _ = each;\n",
    tests: ["test/crates.test.ts", "-t", "size of its consumer's type"],
  },
  {
    name: "bound-impl-facts-unasked",
    breaks:
      "a caller giving an impl's dictionary for a callee's bound, `C: Push<T>` of a `Stack<T>`, isn't asked what the impl asks of `T`",
    file: "crates/rust-js-lower/src/analysis/type_facts/calls.rs",
    find: "        for bound in bounds(tcx, foreign, given, callee) {\n",
    replace:
      "            for bound in bounds(tcx, foreign, given, callee).into_iter().filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^impl_type_facts_through_trait.rs$"],
  },
  {
    name: "unsized-name-unasked",
    breaks: "`type_name` of a `T: ?Sized` is asked of none, and refused",
    file: "crates/rust-js-lower/src/analysis/type_facts/calls.rs",
    find: "                of.is_sized(tcx, typing_env) || !matches!(fact, TypeFact::Size | TypeFact::Room | TypeFact::Align)\n",
    replace: "                of.is_sized(tcx, typing_env)\n",
    tests: ["test/corpus.test.ts", "-t", "^type_names_generic.rs$"],
  },
  {
    name: "held-param-name-unasked",
    breaks:
      "`type_name` of a type that holds a type parameter asks none of its name",
    file: "crates/rust-js-lower/src/analysis/type_facts/calls.rs",
    find: "            if fact == TypeFact::Name\n                && let Some(of) = args.types().last()\n",
    replace:
      "            if false\n                && let Some(of) = args.types().last()\n",
    tests: ["test/corpus.test.ts", "-t", "^type_names_generic.rs$"],
  },
  {
    name: "phantom-bound-unnamed",
    breaks:
      "`{:?}` of a `PhantomData<T>` written in place is given no name of `T`, and refused",
    file: "crates/rust-js-lower/src/analysis/type_facts/calls.rs",
    find: "            for index in debug_named(tcx, bound) {\n",
    replace:
      "            for index in debug_named(tcx, bound).into_iter().take(0) {\n",
    tests: ["test/corpus.test.ts", "-t", "^phantom_debug.rs$"],
  },
  {
    name: "std-callee-not-followed",
    breaks:
      "a std body compiled with the crate is asked what its manifest says, which has none yet, not followed as the crate's own are: std's `fuse` doesn't ask what its `Fuse`'s `next` asks, `IFused`",
    file: "crates/rust-js-lower/src/analysis/type_facts/calls.rs",
    find: "        if !callee.is_local() && !fns.contains_key(&callee) {\n",
    replace: "        if !callee.is_local() {\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "impl-method-facts-not-asked",
    breaks:
      "a library impl method's facts, `Fuse<I>`'s `next_back`'s `IFused`, aren't asked of a caller of the trait's method, which gives `false`: a `FusedIterator`'s `Fuse` stepped from its back drops it as it ends",
    file: "crates/rust-js-lower/src/analysis/type_facts/calls.rs",
    find: "                && instance.def_id() != callee\n",
    replace: "                && false\n",
    tests: ["test/corpus.test.ts", "-t", "^std_fuse.rs$"],
  },
  {
    name: "stepped-iterator-impl-facts-asked",
    breaks:
      "a caller asks the facts of `Fuse<I>`'s `next` impl, stepped as a JS iterator, so its impl methods take an `IFused` nothing reads",
    file: "crates/rust-js-lower/src/analysis/type_facts/calls.rs",
    find: "            if !crate::recognition::stepped_as_js_iterator(tcx, callee, args)\n",
    replace: "            if true\n",
    tests: ["test/corpus.test.ts", "-t", "^fused_iterators.rs$"],
    snapshots: true,
  },
  {
    name: "specialized-facts-unfollowed",
    breaks:
      "what both impls a fact picks between ask, `FlattenCompat`'s `size_hint`'s `ConstSize`, isn't asked of a `Flatten`'s, and std's compile refuses it",
    file: "crates/rust-js-lower/src/analysis/type_facts/calls.rs",
    find: "            if let Some(spec) = fact_specialization(tcx, typing_env, callee, args)\n                && fns.contains_key(&spec.selected.0)\n",
    replace:
      "            if let Some(spec) = fact_specialization(tcx, typing_env, callee, args).filter(|_| false)\n                && fns.contains_key(&spec.selected.0)\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "impl-of-a-slot-type-unasked",
    breaks:
      "a caller giving an impl's dictionary for a type of its slots that isn't one of its type parameters, `impl<T: Iterator> Itertools for T` of `<I as IntoIterator>::IntoIter`, asks none of what the impl asks, and is refused, as itertools' `free::sorted` and semver's were (#794)",
    file: "crates/rust-js-lower/src/analysis/type_facts/calls.rs",
    find: "            true if given_params => self.calls.push((caller, impl_id, impl_args)),\n",
    replace: "            true if given_params => {}\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_collect_of_a_built_type_relayed.rs$"],
  },
  {
    name: "std-method-facts-unrelayed",
    breaks:
      "the analysis asks a std callee only its manifest's facts, none of std's, so `impl Extend for Either`'s `inner.extend(iter)` through `L: Extend<A>` asks its `T::IntoIter`'s in-place buffer of no caller, refused (#794)",
    file: "crates/rust-js-lower/src/analysis/type_facts/calls.rs",
    find: "                    true => crate::traits::foreign_facts(tcx, foreign, id),\n",
    replace: "                    true => foreign.type_facts(id),\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_extend.rs$"],
  },
  {
    name: "std-method-facts-of-resolved-calls",
    breaks:
      "a std trait's required method rustc resolves to std's own impl, `Vec::from_iter(it)`, relays its declaration's facts too, giving `sorted(it, IInPlace)` a parameter nothing reads (#794)",
    file: "crates/rust-js-lower/src/analysis/type_facts/calls.rs",
    find: "            let resolved = named.len() > 1;\n",
    replace: "            let resolved = false && named.len() > 1;\n",
    tests: ["test/corpus.test.ts", "-t", "^from_iter_called.rs$"],
    snapshots: true,
  },
  {
    name: "bound-collect-unrelayed",
    breaks:
      "a `collect()` into what the caller's own bound collects into, `Result<U, E>: FromIterator<..>`, asks nothing of the iterator, though the lowering calls its dictionary's `from_iter` with its in-place buffer: `try_collect` refused (#794)",
    file: "crates/rust-js-lower/src/analysis/type_facts/calls.rs",
    find: "                if bound.def_id == from_iterator && Some(callee) == collect_of(tcx) {\n",
    replace: "                if false && bound.def_id == from_iterator && Some(callee) == collect_of(tcx) {\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_accumulations.rs$"],
  },
  {
    name: "library-impl-associated-slot-unrelayed",
    breaks: "a library impl's fact of a std associated type of its parameter, `<T as Iterator>::Item` of `impl<T: Iterator> Tools for T`, isn't asked of a generic caller giving `<I as Iterator>::Item`, so its dictionary is refused, \"`size_of` of an associated type whose facts its caller's dictionary doesn't carry\" (#1401)",
    file: "crates/rust-js-lower/src/analysis/type_facts/calls.rs",
    find: "        let relayed = |at: usize| of_slots(at) || at >= impl_args.len();\n",
    replace: "        let relayed = |at: usize| of_slots(at);\n",
    tests: ["test/crates-pairs-3.test.ts", "-t", "two crates: library_default_given_an_associated_types_size$"],
  },
  {
    name: "dictionary-of-callers-params-relays-no-layout",
    breaks: "generic code that makes an impl's dictionary of its own type parameters, `X2<W, u8>`, relays none of the layouts of the types the impl builds of them, and is refused (#1430)",
    file: "crates/rust-js-lower/src/analysis/type_facts/calls.rs",
    find: "            true if given_params => self.calls.push((caller, impl_id, impl_args)),\n",
    replace: "            true if given_params && impl_args.iter().any(|arg| arg.as_type().is_some_and(|ty| !matches!(ty.kind(), ty::Param(_)) && ty.has_param())) => self.calls.push((caller, impl_id, impl_args)),\n",
    tests: ["test/corpus.test.ts", "-t", "^supertrait_dictionary_asks_its_makers.rs$"],
  },
  {
    name: "type-name-of-an-alias-unasked",
    breaks: "a type's name asks no name of an associated type only a caller knows in it, `Option<I::Item>`'s `IItemName`, so its `type_name` is refused (#1603)",
    file: "crates/rust-js-lower/src/analysis/type_facts/calls.rs",
    find: "                        ty::Alias(..) if is_unknown_where(tcx, part, typing_env) => {\n",
    replace: "                        ty::Alias(..) if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^type_name_of_an_associated_type_within.rs$"],
  },
  {
    name: "trait-method-facts-unrelayed",
    breaks: "a generic call of a library trait's generic method through a dictionary, given the caller's own type parameter, asks the caller's callers none of the facts the contract lists of it, so its size is refused where native Rust's call runs (#2285)",
    file: "crates/rust-js-lower/src/analysis/type_facts/calls.rs",
    find: "                    true => crate::traits::facts_through_dictionary(tcx, foreign, id, facts),\n",
    replace: "                    true => facts,\n",
    tests: ["test/crates-pairs-2.test.ts", "-t", "library_trait_method_fact_through_a_dictionary"],
  },
  {
    name: "size-of-built-type-unasked",
    breaks: "`size_of::<Self>()` of an `impl<T> Sz for Wrap<T>` over `[T; 2]`, a type built of its parameters, asks its callers nothing, so it's refused as `size_of` of a type parameter (#2287)",
    file: "crates/rust-js-lower/src/analysis/type_facts/calls.rs",
    find: "                && let Some(each @ Asked::Built(..)) = asked_of(tcx, foreign, typing_env, caller, (of, fact))\n",
    replace: "                && let Some(each @ Asked::Built(..)) = None::<Asked<'tcx>>\n",
    tests: ["test/corpus.test.ts", "-t", "^size_of_a_type_built_of_an_impls_parameters\\.rs$"],
  },
  {
    name: "held-iterator-facts-unasked",
    breaks:
      "a call making a JS iterator of the crate's iterator holding the caller's `I`, `total(pool)` of a `Pool<I>`, relays nothing its impl asks, its `next`'s `I: TrustedLen` of an `extend(it.by_ref().take(1))`: refused (#2327)",
    file: "crates/rust-js-lower/src/analysis/type_facts/calls.rs",
    find: "            if let Some(evidence) = evidence_impl(tcx, typing_env, bound) {\n",
    replace: "            if let Some(evidence) = evidence_impl(tcx, typing_env, bound).filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_held_by_generic_code_asking_trusted_len.rs$"],
  },
  {
    name: "dispatched-overrides-unrelayed",
    breaks: "a dispatched call's overrides' asks of the method's own parameters aren't relayed to its caller, so a generic caller whose `B` an own `collect` sizes isn't given `BSize` and is refused it (#2449)",
    file: "crates/rust-js-lower/src/analysis/type_facts/calls.rs",
    find: "                self.dispatched.push((caller, each, (callee, args)));\n",
    replace: "                let _ = (each, caller);\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_own_collect_asking_its_outputs_size.rs$"],
  },
];
