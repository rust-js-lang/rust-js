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
    breaks: "a raw pointer's type is refused as any type is, not as addressable memory",
    file: "src/lower/refusal.rs",
    find: "        Refused::Type(_, ty) => is_memory_type(tcx, ty),\n",
    replace: "        Refused::Type(_, _) => false,\n",
    tests: ["test/diagnostics.test.ts", "-t", "unsupported type"],
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
    name: "refusal-uncompiled-unsaid",
    breaks: "a call into a crate of crates.io's rust-js couldn't compile doesn't say why it isn't compiled",
    file: "src/lower/refusal.rs",
    find: "        error = error.with_note(format!(\n            \"its crate is its types alone",
    replace: "        let _ = (format!(\n            \"its crate is its types alone",
    tests: ["test/registry-crates.test.ts", "-t", "types alone"],
  },
];
