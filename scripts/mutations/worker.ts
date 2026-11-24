// Mutations of src/worker.rs (ADR 0093): a case's builds compiled in one
// rustc session, each answered as its own process would be (#2301, ADR 1182).
import type { Mutation } from "../mutations";

const tested = ["test/worker.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "worker-goes-on-after-printing",
    breaks: "a build after one that printed is lowered in the session anyway: a program after its refused library sees the library's errors and is refused, where its own compile isn't (#2301)",
    file: "src/worker.rs",
    find: "            if self.capture.printed() {\n",
    replace: "            if false && self.capture.printed() {\n",
    tests: tested,
  },
  {
    name: "worker-one-segment",
    breaks: "a later build's lowering prints into the build before's output: a refused library's errors are the program's, and the library's own output is empty (#2301)",
    file: "src/worker.rs",
    find: "            if let Err(error) = self.capture.next() {\n",
    replace: "            if let Err(error) = Ok::<(), String>(()) {\n",
    tests: tested,
  },
  {
    name: "worker-later-builds-unrecorded",
    breaks: "a build after the first is given no record of what the session read, and fails to write its own (#2301)",
    file: "src/worker.rs",
    find: "                std::fs::copy(&record, staged).map_err(|error| format!(\"{}: {error}\", staged.display()))?;\n",
    replace: "                _ = (&record, staged);\n",
    tests: tested,
  },
  {
    name: "worker-panic-unsaid",
    breaks: "a panic that kills the worker leaves its message in the build's capture, which it removes: the harness saw `it ended (101)` and nothing of why (#2448)",
    file: "src/worker.rs",
    find: "        if std::thread::panicking() {\n",
    replace: "        if false && std::thread::panicking() {\n",
    unitTests: "worker::tests::a_panic_past_a_capture_says_what_its_build_printed",
  },
  {
    name: "worker-shares-any-flags",
    breaks: "builds with flags of rust-js's besides what they write share a session, whose flags are the process's statics, so one build's flag changes another's JS (#2301)",
    file: "src/worker.rs",
    find: "    if kept.len() != 1 || kept[0].starts_with('-') {\n",
    replace: "    if kept.is_empty() {\n",
    unitTests: "worker::tests::builds_share_a_session_where_they_differ_only_in_what_they_write",
  },
];
