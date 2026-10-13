// Mutations of src/lower/mut_refs.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "nested-mut-object-accepted",
    breaks: "`to_vec(c)` of an object gives a `Vec` of handles, which the caller takes for the objects",
    file: "src/lower/mut_refs.rs",
    find: "        if let Some(here) = self.nested_mut_object(def_id, generic_args) {\n",
    replace: "        if let Some(here) = None::<Ty<'tcx>> {\n",
    tests: ["test/diagnostics.test.ts","-t","generic &mut T"],
  },
  {
    name: "nested-mut-object-fields-unread",
    breaks: "a generic struct's `r: &mut T` field, built of an object by the caller, is read as a cell",
    file: "src/lower/mut_refs.rs",
    find: "                    todo.extend(adt.all_fields().map(|field| self.field_ty(field, args)));",
    replace: "                    todo.extend(adt.all_fields().map(|field| self.field_ty(field, args)).filter(|_| false));",
    tests: ["test/diagnostics.test.ts","-t","in a struct"],
  },
  {
    name: "nested-mut-object-bounds-unread",
    breaks: "`apply(c, |x| x.n += 1)` gives the closure a box of the object, and `x.n` is the box's",
    file: "src/lower/mut_refs.rs",
    find: "        for (clause, _) in self.tcx.clauses_of(def_id).instantiate_identity(self.tcx) {\n",
    replace: "        for (clause, _) in self.tcx.clauses_of(def_id).instantiate_identity(self.tcx).into_iter().filter(|_| false) {\n",
    tests: ["test/diagnostics.test.ts","-t","given to a closure"],
  },
  {
    name: "generic-result-kept-as-cell",
    breaks: "`pick(true, &mut c, &mut d).n += 5` of an object `T` writes `n` on the handle, not on `c`",
    file: "src/lower/mut_refs.rs",
    find: "self.instantiated(pointee, generic_args)) =>\n            {\n                Expr::member(value, \"value\")\n",
    replace: "self.instantiated(pointee, generic_args)) =>\n            {\n                value\n",
    tests: ["test/corpus.test.ts","-t","generic_mut_ref_kept"],
  },
  {
    name: "map-handles-unsorted",
    breaks: "a B-tree's `iter_mut()` and `values_mut()` go in insertion order, not its keys'",
    file: "src/lower/mut_refs.rs",
    find: "            Std::Map(MapOp::Iter(_)) if self.is_sorted(receiver) => {",
    replace: "            Std::Map(MapOp::Iter(_)) if false && self.is_sorted(receiver) => {",
    tests: ["test/corpus.test.ts", "-t", "std_mut_items"],
  },
  {
    name: "cow-mut-no-handle",
    breaks: "`to_mut()` of a `Cow<str>` is refused",
    file: "src/lower/mut_refs.rs",
    find: '            Std::Cow(CowOp::ToMut) => (Helper::CowMut, "$cowMut"),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^cows"],
  },
  {
    name: "mut-items-box-unseen",
    breaks: "a `Box<[T; N]>`'s `&mut` items, `for n in &mut boxed`, aren't a sequence's",
    file: "src/lower/mut_refs.rs",
    find: "        let receiver = receiver.boxed_ty().unwrap_or(receiver);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "boxed_array_iter"],
  },
  {
    name: "mut-items-pointee-unseen",
    breaks: "a `Box<[T; N]>`'s `into_iter()` through a `&mut` makes no handles",
    file: "src/lower/mut_refs.rs",
    find: "            Std::Same | Std::Pointee if sequence => (Helper::MutItems, \"$mutItems\"),\n",
    replace: "            Std::Same if sequence => (Helper::MutItems, \"$mutItems\"),\n",
    tests: ["test/corpus.test.ts", "-t", "boxed_array_iter"],
  },
];
