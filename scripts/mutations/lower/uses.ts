// Mutations of src/lower/uses.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "list-names-unread",
    breaks: "a `use` list's names, one item since 1.100 whose tree nests them, aren't leaves, so `pub use cache_server::{cache, unstable_cache}` re-exports neither (ADR 0109, amended)",
    file: "src/lower/uses.rs",
    find: "                walk(item, nested, *def_id, &path.segments, found);\n",
    replace: "",
    tests: ["test/modules.test.ts", "-t", "re-exported as written"],
  },
  {
    name: "list-self-named-self",
    breaks: "a list's `self`, `pub use foo::bar::{self}`, keeps its `self` segment and name, so it's re-exported as `self`, not `bar` (ADR 0109, amended)",
    file: "src/lower/uses.rs",
    find: "            path.segments.pop();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^module_of_types_reexported.rs$"],
    snapshots: true,
  },
];
