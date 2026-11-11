// Mutations of src/lower/recognition/type_facts.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "class-key-unpeeled",
    breaks:
      "a map or a set keyed by a `&T` of a type parameter is refused, its class taken for the reference's, not `T`'s (#313)",
    file: "src/lower/recognition/type_facts.rs",
    find: "        ty::Ref(_, inner, rustc_ast::Mutability::Not) => class_key(inner),\n",
    replace: "        ty::Ref(_, _, rustc_ast::Mutability::Not) => key,\n",
    tests: ["test/corpus.test.ts", "-t", "^map_keyed_by_a_params_ref.rs$"],
  },
  {
    name: "needs-drop-unknown",
    breaks: "`mem::needs_drop::<T>()` is refused, not rustc's answer",
    file: "src/lower/recognition/type_facts.rs",
    find: '        "std::mem::needs_drop" => return &[TypeFact::NeedsDrop],\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^needs_drop.rs$"],
  },
  {
    name: "align-of-val-unknown",
    breaks: "`align_of_val(&x)`, which has no diagnostic name, is refused",
    file: "src/lower/recognition/type_facts.rs",
    find: '        _ if std_path(tcx, def_id) == "std::mem::align_of_val" => (TypeFact::Align, true),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^size_of.rs$"],
  },
  {
    name: "size-of-val-as-align",
    breaks: "`size_of_val(&x)` is its alignment",
    file: "src/lower/recognition/type_facts.rs",
    find: '        _ if diagnostic("mem_size_of_val") => (TypeFact::Size, true),\n',
    replace:
      '        _ if diagnostic("mem_size_of_val") => (TypeFact::Align, true),\n',
    tests: ["test/corpus.test.ts", "-t", "^size_of.rs$"],
  },
  {
    name: "layout-new-asks-nothing",
    breaks:
      "`Layout::new::<T>()` of a type parameter asks no size or alignment of its caller, and is refused",
    file: "src/lower/recognition/type_facts.rs",
    find: '        "std::alloc::Layout::new" => return &[TypeFact::Size, TypeFact::Align],\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^layout_new.rs$"],
  },
  {
    name: "btree-of-param-classless",
    breaks:
      "a `BTreeSet` of a type parameter's keys is refused, its class not asked of its caller",
    file: "src/lower/recognition/type_facts.rs",
    find: "        _ if is(StdItem::BTreeSet) => TypeFact::TreeSet,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^generic_btree_keys.rs$"],
  },
  {
    name: "tree-fact-hashed",
    breaks:
      "a generic `BTreeMap` is given its caller's `HashMap` class, a `Map` of keys whose `Ord` is the crate's own",
    file: "src/lower/recognition/type_facts.rs",
    find: "        [[TypeFact::Map, TypeFact::Set], [TypeFact::TreeMap, TypeFact::TreeSet]][usize::from(sorted)][usize::from(set)]\n",
    replace:
      "        [[TypeFact::Map, TypeFact::Set], [TypeFact::Map, TypeFact::Set]][usize::from(sorted)][usize::from(set)]\n",
    tests: ["test/corpus.test.ts", "-t", "^btree_own_ord.rs$"],
  },
  {
    name: "iterator-const-size-unknown",
    breaks: "a type its bounds make an `Iterator` is taken for one that may be an array, so its constant size is asked of callers that don't give it, and refused (#1341)",
    file: "src/lower/recognition/type_facts.rs",
    find: "    known_by_bounds(ty) && trait_holds_in(tcx, typing_env, ty, std_item(tcx, StdItem::Iterator)) != Some(true)\n",
    replace: "    known_by_bounds(ty)\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_passed_through_a_chain_of_impls.rs$"],
  },
];
