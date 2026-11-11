// Mutations of src/lower/instances/failing.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "failing-writer-given-uncompiled",
    breaks:
      "a consumer that gives a library's generic writer what may fail calls the library's own, which takes each write to succeed: a file opened to read throws `Bad file descriptor` past `emit` where Rust's is its `Err` (#1392)",
    file: "src/lower/instances/failing.rs",
    find: "    if !failing.any || foreign.fails(callee) || !args.types().next().is_some() {\n",
    replace: "    if true {\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_generic_writer_given_a_failing_one$"],
  },
  {
    name: "failing-writer-given-to-a-helper-unasked",
    breaks:
      "a library's generic function that writes only through a helper of its library's isn't asked of the helper: `emit_through` given a writer that refuses is `true` (#1392)",
    file: "src/lower/instances/failing.rs",
    find: "                || (compiled_here(self.foreign, tcx, callee) && !self.foreign.fails(callee) && self.fails((callee, given), seen))\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "two crates: library_generic_writer_given_a_failing_one$"],
  },
  {
    name: "failing-writer-given-to-a-closure-unasked",
    breaks:
      "a library's method that writes only in a closure it makes, `filter(|line| self.put(..).is_ok())`, isn't asked of the closure: all the writes count where Rust's count those taken (#1392)",
    file: "src/lower/instances/failing.rs",
    find: "                    Some(ty::Closure(closure, of)) => !closure.is_local() && self.fails((closure, of), seen),\n",
    replace: "                    Some(ty::Closure(..)) => false,\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_method_of_a_generic_writer_given_a_failing_one$"],
  },
  {
    name: "failing-value-formatted-unasked",
    breaks:
      "a library's generic formatter given a value whose `Display` fails isn't compiled here: `write!(out, \"[{t}]\")` tested as `Ok` throws past it (#1392)",
    file: "src/lower/instances/failing.rs",
    find: "            return (args.types().next().into_iter().flat_map(formatted_parts))\n                .any(|part| may_fail(fmt, tcx.mk_args(&[part.into()])));\n",
    replace: "            return false;\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_generic_formatter_given_a_failing_one$"],
  },
];
