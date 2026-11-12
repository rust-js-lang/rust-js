// Mutations of src/lower/recognition/specialization.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "held-facts-unreduced",
    breaks:
      "`TrustedLen` of a `&mut I` isn't `I`'s, as std's impl for it asks: no caller gives it, refused (ADR 0975)",
    file: "src/lower/recognition/specialization.rs",
    find: "    let [(imp, params)] = &impls[..] else {\n",
    replace: "    let [(imp, params), _] = &impls[..] else {\n",
    tests: ["test/corpus.test.ts", "-t", "^copied_clone_of_its_own.rs$"],
  },
  {
    name: "never-returning-default-picked",
    breaks:
      "a default that only panics, std's general `try_get_unchecked`, is compiled as one the facts pick: its inlined `panic!` message is a raw pointer, refused (ADR 0975)",
    file: "src/lower/recognition/specialization.rs",
    find: "    if never_returns(tcx, selected) {\n",
    replace: "    if false && never_returns(tcx, selected) {\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "zip-specialization-unfacted",
    breaks:
      "a zip's sides' `TrustedRandomAccess` picks no impl: std's `ZipImpl` is left to no fact, so std's compiled `Zip` is refused (ADR 0975)",
    file: "src/lower/recognition/specialization.rs",
    find: '        ("ZipImpl", TypeFact::RandomAccess),\n',
    replace: "",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "assume-count-specialized",
    breaks:
      "std's `Filter::count` asks `SpecAssumeCount`'s `TrustedLen` impl, which rust-js refuses: std's bodies don't compile (ADR 0932)",
    file: "src/lower/recognition/specialization.rs",
    find: '    "SpecAssumeCount",\n',
    replace:
      '',
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "fused-specialization-unseen",
    breaks:
      "a std-private trait's method whose impl rustc leaves to the caller's type, `FuseImpl::next` of a `Fuse<I>`, isn't seen to be picked by `I` being a `FusedIterator`, and is refused",
    file: "src/lower/recognition/specialization.rs",
    find: "        .filter(|&other| {\n            other != impl_id\n",
    replace:
      "        .filter(|&other| {\n            false\n                && other != impl_id\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "step-by-specializations-unoptimizing",
    breaks:
      "std's `StepBy` range specializations aren't read as optimizations, so its `SpecRangeSetup::setup` can't be called in generic code (ADR 0917)",
    file: "src/lower/recognition/specialization.rs",
    find: '    "SpecRangeSetup",\n    "StepByImpl",\n    "StepByBackImpl",\n',
    replace: '',
    tests: [
      "test/std-package.test.ts",
      "-t",
      "the committed @rust-js/std is std's bodies",
    ],
  },
  {
    name: "in-place-collect-any-size",
    breaks:
      "a `Vec` collect of items bigger than its source's, which std collects otherwise than in place, drops what it collected first where a panic stops it (#423)",
    file: "src/lower/recognition/specialization.rs",
    find: "&& src.align.abi == dest.align.abi && src.size >= dest.size",
    replace: "&& src.align.abi == dest.align.abi",
    tests: ["test/corpus.test.ts", "-t", "^panic_part_way_drops.rs$"],
  },
  {
    name: "fused-may-collect-in-place",
    breaks:
      "generic code's `src.map(f).fuse().collect()` is taken to be one std may collect in place, where no `Fuse` is `InPlaceIterable`, so it's refused or drops what it collected first (#672)",
    file: "src/lower/recognition/specialization.rs",
    find: "known_by_bounds(held) || trait_holds_in(tcx, typing_env, held, trait_id) != Some(false))",
    replace: "known_by_bounds(held) || true)",
    tests: ["test/corpus.test.ts", "-t", "^generic_collect_drops_rest_first.rs$"],
  },
  {
    name: "in-place-source-unkept",
    breaks:
      "generic code's collect of `src.map(f)` is taken to be of no source std collects in place, so over a `Vec`'s `IntoIter` a panic drops what's left first (#672)",
    file: "src/lower/recognition/specialization.rs",
    find: "        return Some(InPlaceSource::Of(held));\n",
    replace: "        return Some(InPlaceSource::Never);\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_collect_drops_rest_first.rs$"],
  },
  {
    name: "rc-eq-asked-of-none",
    breaks: "a generic `==` of `Rc`s is never given its caller's `TEq`, so it's refused as of a type parameter no caller gives (#931)",
    file: "src/lower/recognition/specialization.rs",
    find: "        pointees(tcx, of, &mut found);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^rc_eq_generic_by_address_first.rs$"],
  },
  {
    name: "rc-eq-asked-through-no-std-type",
    breaks: "`contains` of a `Vec<Rc<T>>`'s items, `Rc<T>: PartialEq` inside a `Vec`, gives no `TEq`, so it's refused (#931)",
    file: "src/lower/recognition/specialization.rs",
    find: "                args.types().for_each(|arg| pointees(tcx, arg, found));\n",
    replace: "",
    tests: ["test/lowering-std-unwinds.test.ts", "-t", "generic code asks"],
  },
  {
    name: "source-iter-step-not-through-some",
    breaks: "a `Fuse`'s `as_inner`, which gives on the borrow it made a `Some` of, has no step, which std's compile asserts (#852)",
    file: "src/lower/recognition/specialization.rs",
    find: "        ) if matches!(**kind, mir::AggregateKind::Adt(_, made, ..) if made == *variant) => {\n",
    replace: "        ) if false => {\n",
    tests: ["test/std-package.test.ts", "-t", "committed @rust-js/std is std's bodies"],
  },
  {
    name: "ref-const-size-not-its-pointees",
    breaks: "a flatten's items' constant size, of a reference to a type only a caller knows, `&I`'s, is refused, \"which no caller gives\", where it's `I`'s, as std's `ConstSizeIntoIterator` gives a `&[T; N]`'s (#1273)",
    file: "src/lower/recognition/specialization.rs",
    find: "        && known_by_bounds(pointee)\n    {\n        return vec![(pointee, fact)];\n",
    replace: "        && false\n    {\n        return vec![(pointee, fact)];\n",
    tests: ["test/corpus.test.ts", "-t", "^flat_map_of_a_bound_reference.rs$"],
  },
  {
    name: "vec-collect-of-a-built-type-asks-no-in-place",
    breaks: "`IVecCollect` of an adapter generic code makes of its type parameter's iterator asks its caller no `IInPlace`, so its collect is refused (#1239)",
    file: "src/lower/recognition/specialization.rs",
    find: "        if in_place.is_none() {\n            facts.extend(held_facts(tcx, typing_env, of, TypeFact::InPlace));\n",
    replace: "        if false {\n            facts.extend(held_facts(tcx, typing_env, of, TypeFact::InPlace));\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_collect_as_its_caller_knows.rs$"],
  },
  {
    name: "vec-collect-of-a-built-type-asks-no-trusted-len",
    breaks: "`IVecCollect` of an adapter generic code makes of its type parameter's iterator asks its caller no `ITrustedLen`, so its collect is refused (#1239)",
    file: "src/lower/recognition/specialization.rs",
    find: "        if in_place != Some(true) {\n            facts.extend(vec_collect_trusted_facts(tcx, typing_env, of));\n",
    replace: "        if false {\n            facts.extend(vec_collect_trusted_facts(tcx, typing_env, of));\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_collect_as_its_caller_knows.rs$"],
  },
  {
    name: "vec-collect-way-fold-by-next",
    breaks: "the compiler's `vec_collect_way` says a `TrustedLen` not collected in place is collected by `next`, which the runtime's `$vecCollectWay` and std's `SpecFromIter` fold (#1239)",
    file: "src/lower/recognition/specialization.rs",
    find: "        (false, true) => \"fold\",\n",
    replace: "        (false, true) => \"next\",\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_collect_as_its_caller_knows.rs$"],
  },
  {
    name: "specialization-generics-counted",
    breaks: "a more specific impl's generics are counted, not matched kind by kind, so `SpecReadByte`'s `StdinLock<'_>` impl, a lifetime, is instantiated with `R`'s type, and the compiler crashes (#1156)",
    file: "src/lower/recognition/specialization.rs",
    find: "                && same_kinds_of_generics(tcx, other, impl_id)\n",
    replace: "                && tcx.generics_of(other).count() == tcx.generics_of(impl_id).count()\n",
    tests: ["test/corpus.test.ts", "-t", "^io_read_chain_take_bytes.rs$"],
  },
  {
    name: "spec-read-byte-not-optimizing",
    breaks: "`Bytes::next`'s `SpecReadByte` is a specialization that changes results, not one that only optimizes, so `bytes()` is refused (#1156)",
    file: "src/lower/recognition/specialization.rs",
    find: "    \"SpecReadByte\",\n",
    replace: "",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std"],
  },
  {
    name: "tuple-key-class-asked-whole",
    breaks: "a map keyed by a tuple of a type parameter asks its caller the tuple's class, which no caller gives where a blanket impl's copy builds it, so it's refused, \"whose class isn't given\" (#1346)",
    file: "src/lower/recognition/specialization.rs",
    find: "                    Some(parts) => each.extend(parts.types().iter().rev().map(|&part| class_key(part))),\n",
    replace: "                    Some(_) => held.push((key, fact)),\n",
    tests: ["test/corpus.test.ts", "-t", "^map_keyed_by_a_tuple_of_a_type_parameter.rs$"],
  },
  {
    name: "array-key-held-as-its-items",
    breaks: "a map keyed by an array of a type parameter, `[T; N]`, a callee asks of what it's given, holds its item's class once an item, so `[T; 536870907]` of a blanket impl's copy takes the compiler's memory (#1371)",
    file: "src/lower/recognition/specialization.rs",
    find: "                    Some(parts) => each.extend(parts.types().iter().rev().map(|&part| class_key(part))),\n",
    replace: "                    Some(parts) => each.extend(match parts { KeyParts::Array(item) => vec![item; 536870907], KeyParts::Tuple(parts) | KeyParts::Fields(_, parts) => parts }.into_iter().rev().map(class_key)),\n",
    tests: ["test/runtime-package.test.ts", "-t", "a type parameter's items compiles within a gigabyte"],
  },
  {
    name: "never-holding-fact-asked",
    breaks: "a fact a type built of a type parameter can never have, whatever its callers give, `vec::IntoIter<Held<I>>`'s `TrustedRandomAccessNoCoerce`, no impl making `Held<_>` `Copy`, is asked of callers, none of whom gives it, and refused (#1385)",
    file: "src/lower/recognition/specialization.rs",
    find: "        Err(_) if !crate::lower::generic_facts::may_hold_for_some(tcx, ty, trait_id) => Some(false),\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "a zip of what's never random access asks no caller whether it is"],
  },

  {
    name: "other-trait-bound-taken-for-none",
    breaks: "std's one impl of a trait for a type constructor that asks its parameters another trait, `vec::IntoIter<T>`'s `TrustedRandomAccessNoCoerce` of `T: NonDrop`, is taken apart as asking nothing, so no caller is asked whether `vec::IntoIter<I::Item>` is one, and the zip is refused (#1389)",
    file: "src/lower/recognition/specialization.rs",
    find: "                && trait_holds_in(tcx, typing_env, held[at], bound.def_id()) != Some(true)\n",
    replace: "                && false\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_zip_of_a_vec_of_a_slots_items"],
  },
];
