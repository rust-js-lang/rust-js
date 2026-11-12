// Mutations of src/lower/analysis/fresh_strings.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "fresh-string-touched-unseen",
    breaks: "a `String` read or written before the call is still proved fresh-empty, so a crate's reader sees reads of none of its capacity (#1297)",
    file: "src/lower/analysis/fresh_strings.rs",
    find: "            if touches(string.local, |v| v.visit_statement(statement, location)) {\n                return false;\n",
    replace: "            if touches(string.local, |v| v.visit_statement(statement, location)) {\n                continue;\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "cleared String"
    ]
  },
  {
    name: "fresh-string-past-the-start",
    breaks: "a `String` the body is given, made before it, is proved fresh-empty (#1297)",
    file: "src/lower/analysis/fresh_strings.rs",
    find: "        if block == mir::START_BLOCK {\n            return false;\n",
    replace: "        if block == mir::START_BLOCK {\n            continue;\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "String parameter"
    ]
  },
  {
    name: "fresh-string-borrowck-reads-touch",
    breaks: "a `let`'s `FakeRead` of a `String` is taken as a read of it, so the idiom `let mut s = String::new(); r.read_to_string(&mut s)` of a crate's reader is refused (#1297)",
    file: "src/lower/analysis/fresh_strings.rs",
    find: "            if matches!(\n                statement.kind,\n                StatementKind::FakeRead(..) | StatementKind::PlaceMention(..) | StatementKind::AscribeUserType(..)\n            ) {\n                continue;\n            }\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^read_of_a_crates_own_reader.rs$"
    ]
  },
  {
    name: "fresh-string-any-maker",
    breaks: "a `String` made by any call, `String::from(\">\")`, is proved fresh-empty (#1297)",
    file: "src/lower/analysis/fresh_strings.rs",
    find: "            .is_some_and(|(callee, args)| crate::lower::recognition::makes_empty_string(tcx, callee, args))\n",
    replace: "            .is_some_and(|_| true)\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "String with text"
    ]
  },
  {
    name: "fresh-string-reference-written",
    breaks: "a `String` written through its reference before the call is proved fresh-empty (#1297)",
    file: "src/lower/analysis/fresh_strings.rs",
    find: "        if uses(body, reference) != 2 {\n",
    replace: "        if uses(body, reference) < 2 {\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "written through its reference"
    ]
  },
  {
    name: "fresh-string-moved-in-unseen",
    breaks: "a `String` made fresh in a temporary and moved into a field, `let mut held = (String::new(), 7)`, isn't proved fresh-empty, so a crate's reader read into it is refused (#1297)",
    file: "src/lower/analysis/fresh_strings.rs",
    find: "    moved.projection.is_empty().then_some(*moved)\n",
    replace: "    moved.projection.is_empty().then_some(*moved).filter(|_| false)\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^read_of_a_crates_own_reader.rs$"
    ]
  },
  {
    name: "fresh-string-moved-in-unchecked",
    breaks: "a `String` written in a temporary before it's moved into a field is proved fresh-empty, so a crate's reader sees reads of none of its capacity (#1297)",
    file: "src/lower/analysis/fresh_strings.rs",
    find: "                if !made_empty_before(tcx, body, moved, location) {\n",
    replace: "                if moved.local == string.local && !made_empty_before(tcx, body, moved, location) {\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "held String made of one written"
    ]
  },
];
