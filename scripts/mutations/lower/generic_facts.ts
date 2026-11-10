// Mutations of src/lower/generic_facts.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
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
    find: "                matches!(pointee(declared).map(|p| *p.kind()), Some(ty::Param(_)))",
    replace: "                pointee(declared).is_some()",
    tests: ["test/corpus.test.ts", "-t", "^reference_given_as_its_pointer\\.rs$"],
  },
];
