// Mutations of src/lower/refusal.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "memory-refusal-unsaid",
    breaks: "a refusal of addressable memory, a transmute, says only that its call isn't supported",
    file: "src/lower/refusal.rs",
    find: "        (true, _) => format!(\"rust-js does not support addressable memory yet: {what}\"),\n",
    replace: "        (true, _) => format!(\"rust-js does not support {what} yet\"),\n",
    tests: ["test/diagnostics.test.ts", "-t", "a transmute"],
  },
  {
    name: "memory-type-unsaid",
    breaks: "a union's constant is refused as any type is, not as addressable memory",
    file: "src/lower/refusal.rs",
    find: "        Refused::Type(_, ty) => is_memory_type(tcx, ty),\n",
    replace: "        Refused::Type(_, _) => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^union_const.rs$"],
  },
  {
    name: "memory-feature-unsaid",
    breaks: "a static's raw address is refused as any feature is, not as addressable memory",
    file: "src/lower/refusal.rs",
    find: "        Refused::Memory(_) => true,\n",
    replace: "        Refused::Memory(_) => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^static_raw_address.rs$"],
  },
  {
    name: "refusal-reason-dropped",
    breaks: "a refusal's reason, why a derive of an open union is refused, is left out",
    file: "src/lower/refusal.rs",
    find: "        (false, Refused::Because(_, why)) => format!(\"rust-js does not support {what} yet: {why}\"),\n",
    replace: "",
    tests: ["test/diagnostics.test.ts", "-t", "a derive of an open union"],
  },
  {
    name: "raw-pointer-unsaid",
    breaks: "a raw pointer to a value is refused as \"this value\", not as addressable memory",
    file: "src/lower/mir.rs",
    find: "            Rvalue::RawPtr(..) => return Err(refuse(tcx, span, Refused::Memory(\"a raw pointer to a value\"))),\n",
    replace: "",
    tests: ["test/diagnostics.test.ts", "-t", "a raw pointer to a value"],
  },
];
