// Mutations of src/lower/declarations/parts.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "chain-keys-not-followed",
    breaks: "a name shadowed down the chain, `className`, isn't omitted",
    file: "src/lower/declarations/parts.rs",
    find: "            keys.extend(flattened_keys(tcx, field.ty(tcx, args).skip_normalization()));",
    replace: "            keys.push(field_key(tcx, field));",
    tests: ["test/declarations.test.ts"],
  },
  {
    name: "flattened-ref-keys",
    breaks:
      "a field a flattened reference's struct has too isn't omitted: `extends Html`, which TypeScript rejects",
    file: "src/lower/declarations/parts.rs",
    find: "    let ty::Adt(adt, args) = ty.peel_refs().kind() else {\n",
    replace: "    let ty::Adt(adt, args) = ty.kind() else {\n",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "declarations type what a module exports",
    ],
  },
];
