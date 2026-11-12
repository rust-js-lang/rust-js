// Mutations of src/lower/pointers/rewritten.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "reassigned-pointed-made-again",
    breaks: "a place whose address is taken, assigned whole again, is made another object, at another address, its old pointer reading the old value (#1484)",
    file: "src/lower/pointers/rewritten.rs",
    find: "            (true, false) => Rewritten::InPlace,\n",
    replace: "            (true, false) => Rewritten::Made,\n",
    tests: ["test/corpus.test.ts","-t","^address_kept_when_assigned_again.rs$"],
  },
  {
    name: "dropped-taken-for-moved",
    breaks: "a value with drop glue, which rustc drops before its place is assigned again, is refused as one moved out (#1484)",
    file: "src/lower/pointers/rewritten.rs",
    find: "            (false, true) if !reaches(body, written, at, moved_out) => Rewritten::InPlace,\n",
    replace: "            (false, true) if false => Rewritten::InPlace,\n",
    tests: ["test/corpus.test.ts","-t","^address_kept_when_assigned_again.rs$"],
  },
  {
    name: "moved-written-over",
    breaks: "a place moved out and assigned again is written over the object the value moved holds (#1484)",
    file: "src/lower/pointers/rewritten.rs",
    find: "            (false, true) if !reaches(body, written, at, moved_out) => Rewritten::InPlace,\n",
    replace: "            (false, true) => Rewritten::InPlace,\n",
    tests: ["test/diagnostics.test.ts","-t","moved out and assigned again"],
  },
  {
    name: "field-pointers-unseen",
    breaks: "a pointer to a field, `&raw const s.f`, doesn't make a write of it or of what holds it over what it holds (#1484)",
    file: "src/lower/pointers/rewritten.rs",
    find: "            pointed.push(*of);\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","^address_kept_when_assigned_again.rs$"],
  },
  {
    name: "inner-objects-unkept",
    breaks: "an object inside a place written whole, a pointer into it, is replaced by the value's, its pointer reading the old (#1484)",
    file: "src/lower/pointers/rewritten.rs",
    find: "            for end in written.projection.len() + 1..=p.projection.len() {\n",
    replace: "            for end in p.projection.len() + 1..=p.projection.len() {\n",
    tests: ["test/corpus.test.ts","-t","^address_kept_when_assigned_again.rs$"],
  },
  {
    name: "storage-live-crossed",
    breaks: "a pointer made in a loop's last time round is taken to reach the binding's write this time, so a `for` binding is written over a local that isn't there yet (#1484)",
    file: "src/lower/pointers/rewritten.rs",
    find: "                return (false, true);\n",
    replace: "                continue;\n",
    tests: ["test/corpus.test.ts", "-t", "^address_of_an_object_std_lends_from_its_storage.rs$"],
  },
];
