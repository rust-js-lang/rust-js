// Mutations of crates/rust-js-lower/src/std_impls/in_turn.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "own-clone-never-panics",
    breaks: "a clone of the crate's type is taken to never panic, so a `cloned()` whose clone panics leaves what it collected undropped (#708), and a clone of a `Vec` of them leaves the items it made (#897)",
    file: "crates/rust-js-lower/src/std_impls/in_turn.rs",
    find: "            _ if let Some(imp) = self.recognition().user_impl(tr) => self.impl_may_unwind(imp),\n",
    replace: "            _ if let Some(imp) = self.recognition().user_impl(tr) => false && self.impl_may_unwind(imp),\n",
    tests: ["test/corpus.test.ts", "-t", "^(adapters_own_work_that_panics|clone_unwinds_what_it_made).rs$"],
  },
  {
    name: "derived-clone-never-panics",
    breaks: "a derived `Clone` is taken to never panic, as its fields' clones may, so a clone of a `Vec` of them leaves those it made undropped (#897)",
    file: "crates/rust-js-lower/src/std_impls/in_turn.rs",
    find: "                (adt.all_fields()).any(|f| self.clone_may_panic_in(self.field_ty(f, args), seen))\n",
    replace: "                adt.all_fields().any(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_unwinds_what_it_made.rs$"],
  },
  {
    name: "rc-clone-may-panic",
    breaks: "an `Rc`'s clone, which only counts, is taken to panic as what it holds may, a guard where none is needed (#897)",
    file: "crates/rust-js-lower/src/std_impls/in_turn.rs",
    find: "            ty::Adt(..) if self.is_rc(ty) || self.counted_rc(ty).is_some() || self.counted_weak(ty).is_some() => false,\n",
    replace: "",
    tests: ["test/lowering-made-in-turn.test.ts"],
  },
  {
    name: "made-in-turn-every-part",
    breaks: "every part made in turn with a destructor is held, the last that may panic too, which no later one unwinds past: a guard where none is needed (#897)",
    file: "crates/rust-js-lower/src/std_impls/in_turn.rs",
    find: "            .map(|(i, &(ty, _))| last.is_some_and(|last| i < last) && self.drops(ty) != Drops::Nothing)\n",
    replace: "            .map(|(i, &(ty, _))| last.is_some_and(|last| i <= last) && self.drops(ty) != Drops::Nothing)\n",
    tests: ["test/lowering-made-in-turn.test.ts"],
  },
  {
    name: "made-in-turn-none-held",
    breaks: "no part made in turn is dropped as a later one's clone or default unwinds: a derived `Clone`'s or `Default`'s fields leak (#897, #902)",
    file: "crates/rust-js-lower/src/std_impls/in_turn.rs",
    find: "        let last = parts.iter().rposition(|&(_, may_panic)| may_panic);\n",
    replace: "        let last = parts.iter().rposition(|&(_, may_panic)| may_panic).filter(|_| false);\n",
    tests: ["test/corpus.test.ts", "-t", "^(clone|default)_unwinds_what_it_made.rs$"],
  },
  {
    name: "made-each-unguarded",
    breaks: "a clone of a `Vec` whose item's clone panics leaves the items it made undropped (#897)",
    file: "crates/rust-js-lower/src/std_impls/in_turn.rs",
    find: "        if may_panic && self.drops(ty) != Drops::Nothing {\n",
    replace: "        if false && may_panic && self.drops(ty) != Drops::Nothing {\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_unwinds_what_it_made.rs$"],
  },
  {
    name: "clone-may-panic-asked-each-way",
    breaks: "whether a clone may panic asks a type again for each way to it, its path alone kept, so a chain of enums of four variants holding the one before is asked 4 to the power of its depth times, and twenty deep never compiles (#1821)",
    file: "crates/rust-js-lower/src/std_impls/in_turn.rs",
    find: "        if self.is_copy(ty) || !seen.insert(ty) {\n            return false;\n        }\n",
    replace: "        if self.is_copy(ty) || seen.contains(&ty) {\n            return false;\n        }\n        let seen = &mut { let mut path = seen.clone(); path.insert(ty); path };\n",
    tests: ["test/type-depth.test.ts", "-t", "plain$"],
  },
  {
    name: "refcell-clone-never-panics",
    breaks: "a `RefCell`'s clone is taken never to panic, so a `VecDeque`'s `clone_from` of cells, one mutably borrowed, is `*w = v.clone()`, leaving it as it was, where std's has cleared it and pushed what it cloned before the cell's clone panicked (#2627)",
    file: "crates/rust-js-lower/src/std_impls/in_turn.rs",
    find: "            ty::Adt(..) if self.is_std_type(ty, crate::recognition::StdItem::RefCell) => true,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^vec_deque_clone_from_of_a_borrowed_refcell_keeps_what_it_cloned\\.rs$"],
  },
];
