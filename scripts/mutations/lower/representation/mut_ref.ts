// Mutations of src/lower/representation/mut_ref.rs (ADR 0093): what a
// `&mut` to a type is, `MutRefForm` (ADR 1086).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "generic-mut-object-unboxed",
    breaks:
      "`twice(&mut c)` of a generic `&mut T` gives the object `c`, where `twice` reads a box",
    file: "src/lower/representation/mut_ref.rs",
    find: "        self.param_form(fn_id, i).is_some_and(MutRefForm::is_cell)\n",
    replace: "        self.param_form(fn_id, i) == Some(MutRefForm::Box)\n",
    tests: ["test/corpus.test.ts", "-t", "generic_mut_ref"],
  },
  {
    name: "generic-mut-closure-boxed",
    breaks:
      "`f: &mut F` of an `F: FnMut()` is boxed, `f.value()`, where the closure itself is the `&mut` to it, `f()`: right, as a box is, but not the JS a person writes",
    file: "src/lower/representation/mut_ref.rs",
    find: "                self.tcx.fn_trait_kind_from_def_id(id).is_some()\n                    || is_std_def(self.tcx, id, StdItem::Iterator)",
    replace:
      "                false\n                    || is_std_def(self.tcx, id, StdItem::Iterator)",
    tests: ["test/corpus.test.ts", "-t", "closure_mut_ref"],
    snapshots: true,
  },
  {
    name: "dyn-fn-mut-boxed",
    breaks: "`f: &mut dyn FnMut()` is a box, and `f()` calls `f.value()`",
    file: "src/lower/representation/mut_ref.rs",
    find: "            && !self.is_callable(ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "closure_mut_ref"],
    // The box is a cell of the function, `dynamic({ value() { .. } })`,
    // which runs as the function does: only its snapshot tells them apart.
    snapshots: true,
  },
  {
    name: "generic-mut-not-cell",
    breaks:
      "a generic `&mut T` kept, `result.push(&mut list.value)`, is refused as a value of type `&mut T`",
    file: "src/lower/representation/mut_ref.rs",
    find: "            MutRefForm::Box | MutRefForm::GenericBox => true,\n            MutRefForm::Object | MutRefForm::Guard | MutRefForm::GivenAsItIs | MutRefForm::Itself => false,\n",
    replace: "            MutRefForm::Box => true,\n            MutRefForm::GenericBox | MutRefForm::Object | MutRefForm::Guard | MutRefForm::GivenAsItIs | MutRefForm::Itself => false,\n",
    tests: ["test/corpus.test.ts", "-t", "generic_mut_ref_kept"],
  },
  {
    name: "ref-not-boxable",
    breaks: "a `&mut` to a shared reference isn't a cell, and is refused",
    file: "src/lower/representation/mut_ref.rs",
    find: "        !self.recognition().is_unknown_in(ty, typing_env)\n",
    replace: "        !ty.is_ref() && !self.recognition().is_unknown_in(ty, typing_env)\n",
    tests: ["test/corpus.test.ts", "-t", "mut_ref_to_ref"],
  },
  {
    name: "generic-boxed-in-callers-env",
    breaks:
      "a callee's projection is normalized in its caller's environment, whose parameters are numbered otherwise: rustc crashes",
    file: "src/lower/representation/mut_ref.rs",
    find: "                    .try_normalize_erasing_regions(there, ty::Unnormalized::new_wip(ty))\n",
    replace: "                    .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(ty))\n",
    tests: ["test/corpus.test.ts", "-t", "^deref_projection_generic.rs$"],
  },
  {
    name: "mut-ref-to-mut-ref-refused",
    breaks:
      "a `&mut &mut u32`, arrayvec's `ScopeExitGuard`'s, is refused, where it's a cell of the cell",
    file: "src/lower/representation/mut_ref.rs",
    find: "            return self.mut_ref_form_in(inner, param_env).is_cell() || plain_object;\n",
    replace: "            return plain_object;\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_to_mut_ref.rs$"],
  },
  {
    name: "pinned-ref-boxable",
    breaks:
      "a `&mut` to a `Pin<&mut i32>` is a cell of the pin, so `pin.as_mut()` writes the pin, not the number",
    file: "src/lower/representation/mut_ref.rs",
    find: "            return !self.mut_is_the_pin(ty) && self.is_boxable(pointer, param_env);\n",
    replace: "            return self.is_boxable(pointer, param_env);\n",
    tests: ["test/corpus.test.ts", "-t", "^pin_box.rs$"],
  },
  {
    name: "mut-ref-to-object-uncelled",
    breaks:
      "a `&mut` to a `&mut Money`, an impl for `&mut Money`'s `&mut self`, is refused",
    file: "src/lower/representation/mut_ref.rs",
    find: "            return self.mut_ref_form_in(inner, param_env).is_cell() || plain_object;\n",
    replace: "            return self.mut_ref_form_in(inner, param_env).is_cell();\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_to_object_impl.rs$"],
  },
  {
    name: "mut-ref-to-iterator-celled",
    breaks:
      "a `&mut` to a `&mut` iterator is a cell, where it's the JS iterator, and `it.next` isn't one",
    file: "src/lower/representation/mut_ref.rs",
    find: "                && !self.implements_iterator(inner)\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(iter_by_ref|iter_next_back|dyn_iterators).rs$",
    ],
  },
  {
    name: "rc-mut-ref-is-object",
    breaks:
      "a `&mut` to a counted `Rc` is the `Rc` its owners share, so writing one through it is refused",
    file: "src/lower/representation/mut_ref.rs",
    find: "            && (!self.is_object(ty) || shared)\n",
    replace: "            && !self.is_object(ty)\n",
    tests: ["test/crates.test.ts", "-t", "three crates: rc_generic"],
  },
  {
    name: "given-iterator-written-refused",
    breaks:
      "an iterator given as it is isn't written in place, so `*b = b_orig.clone()` of a `&mut` to one, and `mem::replace` of one a closure captured, are refused",
    file: "src/lower/representation/mut_ref.rs",
    find: "        } else if param_env == self.typing_env.param_env && self.iterator_given_as_it_is(ty) {\n",
    replace: "        } else if false {\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(replace_held_given_iterator|iterator_written_through).rs$",
    ],
  },
  {
    name: "boxable-opaque-unrevealed",
    breaks:
      "a `&mut` of a closure returned as `impl FnMut` is a box of it, `{ value: f }`, which what's lent it calls as the function",
    file: "src/lower/representation/mut_ref.rs",
    find: "        let ty = self.reveal(ty);\n        // A `Pin` is its pointer",
    replace: "        // A `Pin` is its pointer",
    tests: ["test/corpus.test.ts", "-t", "^impl_fn_mut_lent.rs$"],
  },
  {
    name: "type-parameter-mut-not-handle",
    breaks:
      "a `&mut` of a type parameter's item std gives a closure, generic `retain_mut`'s, is the item, where the closure made at the call writes its handle's `value`",
    file: "src/lower/representation/mut_ref.rs",
    find: "        self.mut_ref_form(ty).is_cell() || self.is_unknown(ty)\n",
    replace: "        self.mut_ref_form(ty) == MutRefForm::Box\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(generic_retain_handles|refcell_generic_mut_method).rs$",
    ],
  },
  {
    name: "guard-written-in-place",
    breaks:
      "`mem::swap` of two `RefCell` guards through `&mut`s swaps what the cells hold, where Rust swaps the guards",
    file: "src/lower/representation/mut_ref.rs",
    find: "        } else if self.is_guard(ty) {\n",
    replace: "        } else if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^guard_swap_refused.rs$"],
  },
  {
    name: "flattened-state-mut-the-object",
    breaks:
      "a `&mut` to a class's flattened state is the class's object, so what it's lent to writing it whole `$assign`s the chunk, deleting its `name` and `reads` (#478)",
    file: "src/lower/representation/mut_ref.rs",
    find: "|| self.counted_weak(ty).is_some() || self.is_flattened_state(ty);\n",
    replace: "|| self.counted_weak(ty).is_some();\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_lent_mut.rs$"],
  },
  {
    name: "mut-ref-ts-type-its-pointee",
    breaks: "a `&mut` its JS has as a box is declared as its pointee, `bump(n: number, by: number)`, so a TypeScript caller that follows it gives a number, whose `value` its JS writes (#619): the conformance test's caller throws",
    file: "src/lower/representation/mut_ref.rs",
    find: "            true => held_ts(pointee),\n",
    replace: "            true => pointee,\n",
    tests: ["test/declarations-conformance.test.ts"],
  },
  {
    name: "moved-out-shared",
    breaks:
      "`Cell::from_mut(&mut v).replace(vec![3])` gives `[3]`: what's moved out of an in-place cell is the object its write changes after, not a copy apart (#527)",
    file: "src/lower/representation/mut_ref.rs",
    find: "            true => Expr::call_helper(\"$apart\", vec![old]),\n            false => old,\n",
    replace: "            _ => old,\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_cell_(replaced|moved_out).rs$"],
  },
  {
    name: "moved-out-of-object-shared",
    breaks:
      "`Cell::from_mut(v).replace(..)` of a known `Vec` or struct gives the new value: only a type parameter's moved out apart (#527)",
    file: "src/lower/representation/mut_ref.rs",
    find: "        match self.is_unknown(item) || self.mut_ref_form(item).written_in_place() {\n",
    replace: "        match self.is_unknown(item) {\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_cell_replaced.rs$"],
  },
  {
    name: "flattened-moved-out-the-object",
    breaks:
      "what's moved out of a flattened state whole, `mem::replace(state, ..)` of one lent `{ value: task }`, is its class's object, `const old = state.value;`, which the copy back then writes over, so `old` reads the new variant (#658)",
    file: "src/lower/representation/mut_ref.rs",
    find: "            return self.state_copied(old, item, *adt);\n",
    replace: "            return old;\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_moved_out.rs$"],
  },
  {
    name: "flattened-copy-of-every-key",
    breaks:
      "a flattened state's copy has every variant's keys, so it carries a key an earlier variant left on the object, `{ status: \"rejected\", reason, value }`, which a value made in JS hasn't (the user's decision, ADR 1044)",
    file: "src/lower/representation/mut_ref.rs",
    find: "            let keys: Vec<String> = (variant.fields.iter())\n",
    replace: "            let keys: Vec<String> = (variant.fields.iter().chain(adt.all_fields()))\n",
    tests: ["test/classes-flattened-state.test.ts", "-t", "a copy of its variant's keys"],
  },
  {
    name: "flattened-copy-untested",
    breaks:
      "a flattened state's copy is its first variant's keys whatever its tag, so a `Done` moved out reads `Done { value: undefined }` (#659)",
    file: "src/lower/representation/mut_ref.rs",
    find: "            Expr::cond(test, copy(keys), otherwise)\n",
    replace: "            Expr::cond(test, copy(keys), copy(keys))\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_moved_out.rs$"],
  },
];
