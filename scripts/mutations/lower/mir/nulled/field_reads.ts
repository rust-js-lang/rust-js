// Mutations of crates/rust-js-mir/src/nulled/field_reads.rs (ADR 0093): a struct's
// `RefCell` field read where a test of the same field proved it, nothing
// between but its own reads (#1933, step 1(b)).
import type { Mutation } from "../../../../mutations";

const fieldPaths = ["test/lowering-nullable-tests.test.ts", "-t", "a RefCell field tested and read again"];

export const mutations: Mutation[] = [
  {
    name: "field-read-any-call-between",
    breaks: "a call between the test and the read, which may empty the field, is taken to run nothing, so `$unwrap` is dropped where Rust panics (#1933)",
    file: "crates/rust-js-mir/src/nulled/field_reads.rs",
    find: "                    _ if clone => self.clone_runs_nothing(state, destination.local),\n                    _ => false,\n",
    replace: "                    _ if clone => self.clone_runs_nothing(state, destination.local),\n                    _ => true,\n",
    tests: fieldPaths,
  },
  {
    name: "field-read-root-rebound",
    breaks: "the variable the field is reached from, set to another between the test and the read, `node = b`, is taken to be the one tested (#1933)",
    file: "crates/rust-js-mir/src/nulled/field_reads.rs",
    find: "assign.0.projection.is_empty() && Some(assign.0.local) != read.root()\n",
    replace: "assign.0.projection.is_empty()\n",
    tests: fieldPaths,
  },
  {
    name: "field-read-of-a-getter",
    breaks: "a field of a struct JS gives, whose getter may answer each read anew, is proved by an earlier read (#1933)",
    file: "crates/rust-js-mir/src/nulled/field_reads.rs",
    find: "        if base.variant_index.is_some() || self.fields_read_once(base.ty) {\n",
    replace: "        if base.variant_index.is_some() {\n",
    tests: fieldPaths,
  },
  {
    name: "local-cell-read-past-a-call",
    breaks: "a thread-local `Cell`'s `get()` after a call that may set it, `if CURRENT.get().is_none() { panic!() } run(); CURRENT.get().unwrap()`, is taken to be the one tested, `current` where `run` set it `None` and Rust panics (#1933)",
    file: "crates/rust-js-mir/src/nulled/field_reads.rs",
    find: "                        }\n                        _ => false,\n                    };\n                }\n",
    replace: "                        }\n                        _ => true,\n                    };\n                }\n",
    tests: ["test/lowering-early-exits.test.ts", "-t", "a module variable returned after its test"],
  },
  {
    name: "local-cell-read-unproved",
    breaks: "a thread-local `Cell`'s `get()` right after a test of another `get()` of it is taken for one nothing proved, React's resolveCurrentlyRenderingComponent's `$unwrap(current)` (#1933)",
    file: "crates/rust-js-mir/src/nulled/field_reads.rs",
    find: "            Write::Call(block, func, [key]) if let Some(key) = self.local_cell_got(state, func, &key.node) => {\n",
    replace: "            Write::Call(block, func, [key]) if let Some(key) = self.local_cell_got(state, func, &key.node).filter(|_| false) => {\n",
    tests: ["test/lowering-early-exits.test.ts", "-t", "a module variable returned after its test"],
  },
];
