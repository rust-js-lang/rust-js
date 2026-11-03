// Mutations of src/lower/bindings/flattened.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "keyed-units-many",
    breaks: "an enum of two variants without fields, which no key tells apart, is flattened as one told by its keys (#504)",
    file: "src/lower/bindings/flattened.rs",
    find: "    if units > 1 || units == own.len() {\n",
    replace: "    if units == own.len() {\n",
    tests: ["test/flattened-keyed.test.ts", "-t", "no key tells"],
  },
  {
    name: "keyed-shared-key-tells",
    breaks: "a variant is told by a key another variant has too (#504)",
    file: "src/lower/bindings/flattened.rs",
    find: "        let unique = |key: &String| (own.iter().enumerate()).all(|(j, other)| j == i || !other.contains(key));\n",
    replace: "        let unique = |_: &String| true;\n",
    tests: ["test/flattened-keyed.test.ts", "-t", "no key tells"],
  },
  {
    name: "keyed-test-mark-unread",
    breaks: "a variant's `#[rust_js::test]` field doesn't tell it: `'value' in node` where it's `'getter' in node` (#504)",
    file: "src/lower/bindings/flattened.rs",
    find: "            .filter(|(field, _)| is_test_field(tcx, field.did))\n",
    replace: "            .filter(|_| false)\n",
    tests: ["test/flattened-keyed.test.ts", "-t", "intersection"],
  },
  {
    name: "keyed-params-unseen",
    breaks: "a struct's flattened type parameter is no flattened field, so a generic graph's node of its caller's kind is refused (#504)",
    file: "src/lower/bindings/flattened.rs",
    find: "                ty::Param(param) => found.params.entry(id.to_def_id()).or_default().push(param.index),\n",
    replace: "                ty::Param(_) => {}\n",
    tests: ["test/flattened-keyed.test.ts", "-t", "intersection"],
  },
  {
    name: "keyed-param-given-unread",
    breaks: "an enum a struct's flattened type parameter is given is not told by its keys, so it's refused there (#504)",
    file: "src/lower/bindings/flattened.rs",
    find: "    found.keyed.extend(keyed);\n",
    replace: "    drop(keyed);\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_keyed_param.rs$"],
  },
  {
    name: "keyed-test-by-value",
    breaks: "a keyed enum's variant is tested by its key's value, not its presence, which a `None` leaves `undefined` (#504)",
    file: "src/lower/bindings/flattened.rs",
    find: "    crate::js::Expr::bin(crate::js::Op::In, crate::js::Expr::str(key), subject)\n",
    replace: "    crate::js::Expr::bin(crate::js::Op::LooseNe, crate::js::Expr::member(subject, key), crate::js::Expr::null())\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_keyed_enum.rs$"],
  },
  {
    name: "keyed-contract-unread",
    breaks: "a consumer tags a library's keyed enum its contract lists, and tests its tag (ADR 1071)",
    file: "src/lower/bindings/flattened.rs",
    find: "            if adt.is_enum() && foreign.keyed(adt.did()) {\n",
    replace: "            if false {\n",
    tests: ["test/flattened-keyed.test.ts", "-t", "contract"],
  },
  {
    name: "keyed-foreign-params-unseen",
    breaks: "a library's struct's flattened type parameter flattens nothing a consumer gives it (ADR 1071)",
    file: "src/lower/bindings/flattened.rs",
    find: "                found.params.insert(adt.did(), at);\n",
    replace: "                drop(at);\n",
    tests: ["test/flattened-keyed.test.ts", "-t", "contract"],
  },
];
