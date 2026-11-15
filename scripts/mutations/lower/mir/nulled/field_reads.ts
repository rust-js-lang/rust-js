// Mutations of src/lower/mir/nulled/field_reads.rs (ADR 0093): a struct's
// `RefCell` field read where a test of the same field proved it, nothing
// between but its own reads (#1933, step 1(b)).
import type { Mutation } from "../../../../mutations";

const fieldPaths = ["test/lowering-nullable-tests.test.ts", "-t", "a RefCell field tested and read again"];

export const mutations: Mutation[] = [
  {
    name: "field-read-any-call-between",
    breaks: "a call between the test and the read, which may empty the field, is taken to run nothing, so `$unwrap` is dropped where Rust panics (#1933)",
    file: "src/lower/mir/nulled/field_reads.rs",
    find: "                    _ if clone => self.clone_runs_nothing(state, destination.local),\n                    _ => false,\n",
    replace: "                    _ if clone => self.clone_runs_nothing(state, destination.local),\n                    _ => true,\n",
    tests: fieldPaths,
  },
  {
    name: "field-read-root-rebound",
    breaks: "the variable the field is reached from, set to another between the test and the read, `node = b`, is taken to be the one tested (#1933)",
    file: "src/lower/mir/nulled/field_reads.rs",
    find: " && assign.0.local != read.root,\n",
    replace: ",\n",
    tests: fieldPaths,
  },
  {
    name: "field-read-of-a-getter",
    breaks: "a field of a struct JS gives, whose getter may answer each read anew, is proved by an earlier read (#1933)",
    file: "src/lower/mir/nulled/field_reads.rs",
    find: "        if base.variant_index.is_some() || self.fields_read_once(base.ty) {\n",
    replace: "        if base.variant_index.is_some() {\n",
    tests: fieldPaths,
  },
  {
    name: "field-read-back-through-the-test",
    breaks: "the ways back from the read are followed through the test again, so a loop's body, `node = parent`, is between, and a `while .. is_none()`'s exit proves nothing (#1933)",
    file: "src/lower/mir/nulled/field_reads.rs",
    find: "            if b == test || !behind.insert(b) {\n",
    replace: "            if !behind.insert(b) {\n",
    tests: fieldPaths,
  },
];
