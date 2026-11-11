// Mutations of src/lower/mir/partly_written.rs (ADR 0093): custom MIR's
// use of a local before any write of it whole, refused (#1382).
import type { Mutation } from "../../../mutations";

const refused = (name: string) => ["test/diagnostics.test.ts", "-t", name];
const runs = ["test/diagnostics.test.ts", "-t", "custom MIR that writes a local whole before a part of it compiles and runs"];

export const mutations: Mutation[] = [
  {
    name: "part-written-first-unrefused",
    breaks: "a write of a part of a local nothing wrote whole, `_2.1 = x`, is lowered, its JS writing a name it never declared",
    file: "src/lower/mir/partly_written.rs",
    find: "            false => self.used(place.local),\n",
    replace: "            false => {}\n",
    tests: refused("a part written first"),
  },
  {
    name: "unwritten-read-unrefused",
    breaks: "a pointer to a local nothing wrote, `&raw mut _3`, is lowered, its JS reading a name it never declared",
    file: "src/lower/mir/partly_written.rs",
    find: "    fn visit_local(&mut self, local: Local, _: PlaceContext, _: mir::Location) {\n        self.used(local);\n",
    replace: "    fn visit_local(&mut self, local: Local, _: PlaceContext, _: mir::Location) {\n        let _ = local;\n",
    tests: refused("a pointer to what nothing wrote"),
  },
  {
    name: "storage-end-keeps-written",
    breaks: "a read of a local past its storage's end, `StorageDead(_3); RET = _3`, is lowered",
    file: "src/lower/mir/partly_written.rs",
    find: "                    StatementKind::StorageDead(local) => walk.written[local.index()] = false,\n",
    replace: "                    StatementKind::StorageDead(local) => { let _ = local; }\n",
    tests: refused("a read past its storage's end"),
  },
  {
    name: "parameters-unwritten",
    breaks: "a parameter isn't written as the body starts, so every custom MIR body that reads one is refused",
    file: "src/lower/mir/partly_written.rs",
    find: ".map(|l| (1..=body.arg_count).contains(&l))",
    replace: ".map(|l| (1..body.arg_count).contains(&l))",
    tests: runs,
  },
  {
    name: "one-value-reads-refused",
    breaks: "a read of a `()` nothing wrote, which no write changes, Rustlantis's `dump_var(.., _5, ..)`, is refused",
    file: "src/lower/mir/partly_written.rs",
    find: " && !self.any[local.index()] && ",
    replace: " && ",
    tests: runs,
  },
];
