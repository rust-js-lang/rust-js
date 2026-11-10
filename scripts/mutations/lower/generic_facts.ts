// Mutations of src/lower/generic_facts.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "box-form-unasked",
    breaks:
      "generic code's box of an object's `&mut` isn't a form that differs: a library's function given as an `F` reads its box as the object, and a `fn` pointer gives the instance the object",
    file: "src/lower/generic_facts.rs",
    find: "        [Form::Handle, Form::Box]",
    replace: "        [Form::Handle]",
    tests: ["test/crates.test.ts", "-t", "library_generic_fn_item_forms"],
  },
  {
    name: "reified-object-mut-unboxed",
    breaks:
      "a `fn` pointer of an object's `&mut T` gives the instance the object where it takes generic code's box, `view::<Vec<u8>>` reading `r.value` of the vector",
    file: "src/lower/generic_facts.rs",
    find: "            Form::Box => Expr::call_helper(\"$cellOf\", vec![value]),",
    replace: "            Form::Box => value,",
    tests: ["test/corpus.test.ts", "-t", "^generic_fn_pointer_takes_its_types_forms\\.rs$"],
  },
  {
    name: "shared-number-let-out-allowed",
    breaks:
      "a `&u32` given to generic code that may let it out as a raw pointer compiles, its pointer `$ptrTo(5)` thrown at run time",
    file: "src/lower/generic_facts.rs",
    find: "            .then_some(referent)",
    replace: "            .then_some(referent)\n            .filter(|_| false)",
    tests: ["test/corpus.test.ts", "-t", "^shared_number_given_to_let_a_pointer_out_refused\\.rs$"],
  },
  {
    name: "generic-shared-ref-let-out-refused",
    breaks:
      "generic code's own `&T` given on to code that may let it out as a raw pointer is refused, rust_decimal's `&Self`, where only its callers know `T`",
    file: "src/lower/generic_facts.rs",
    find: "        (!self.is_unknown(referent) && !self.is_object(referent) && !self.pointee_is_itself(referent))",
    replace: "        (!self.is_object(referent) && !self.pointee_is_itself(referent))",
    tests: ["test/corpus.test.ts", "-t", "^reference_given_as_its_pointer\\.rs$"],
  },
  {
    name: "raw-pointer-given-unadapted",
    breaks:
      "a slice's pointer, the array, given for generic code's `*mut T` isn't a handle, and its `value` is read of the array",
    file: "src/lower/generic_facts.rs",
    find: "            Form::Handle => Expr::call_helper(\"$ptrTo\", vec![value]),",
    replace: "            Form::Handle => value,",
    tests: ["test/corpus.test.ts", "-t", "^reference_given_as_its_pointer\\.rs$"],
  },
  {
    name: "raw-pointer-form-of-any-pointee",
    breaks:
      "a raw pointer of a concrete pointee is adapted too, an object's handle wrapped again",
    file: "src/lower/generic_facts.rs",
    find: "                matches!(pointee(declared).map(|p| *p.kind()), Some(ty::Param(_)))\n                    && pointee(instantiated).is_some_and(|p| self.pointee_is_itself(p)",
    replace: "                pointee(declared).is_some()\n                    && pointee(instantiated).is_some_and(|p| self.pointee_is_itself(p)",
    tests: ["test/corpus.test.ts", "-t", "^reference_given_as_its_pointer\\.rs$"],
  },
  {
    name: "param-given-mut-raw",
    breaks:
      "a `&mut` to an object given as a type parameter's value is the object, which an impl for `&mut T` reads as a box, and JS throws",
    file: "src/lower/generic_facts.rs",
    find: "            ty::Param(_) => self.mut_ref_form_in(declared, param_env).is_generic_box(),\n",
    replace: "            ty::Param(_) => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_given_as_type_param.rs$"],
  },
  {
    name: "boxed-through-shared-unpeeled",
    breaks:
      "an impl for `&mut T`'s `&self`, called where its type is known, is given the object, not a box of it",
    file: "src/lower/generic_facts.rs",
    find: "                    (declared, instantiated) = (d, i);\n",
    replace: "                    let _ = (d, i);\n                    break;\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_given_as_type_param.rs$"],
  },
  {
    name: "fn-bound-mut-boxed",
    breaks:
      "a `&mut F` of an `F: FnMut()` is generic code's box too, so a closure given for it is `$cellOf(f)`, which generic code calls",
    file: "src/lower/generic_facts.rs",
    find: "            ty::Ref(..) => self.is_generic_box_in(declared, param_env),\n",
    replace: "            ty::Ref(_, pointee, _) => matches!(pointee.kind(), ty::Param(_)),\n",
    tests: ["test/corpus.test.ts", "-t", "^closure_mut_ref\\.rs$"],
  },
];
