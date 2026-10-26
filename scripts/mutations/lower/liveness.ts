// Mutations of src/lower/liveness.rs (ADR 0093).
import type { Mutation } from "../../mutations";

const binding = ["test/jsx.test.ts", "-t", "a binding needs no allow"];
const clippy = ["test/clippy.test.ts", "-t", "of clean Rust says nothing"];

export const mutations: Mutation[] = [
  {
    name: "js-read-fields-dead",
    breaks: "a struct given a binding has its fields said never read, as rustc alone sees it, so each needs `#[allow(dead_code)]`",
    file: "src/lower/liveness.rs",
    find: "    summary.final_result.live_symbols.extend(read);\n",
    replace: "",
    tests: binding,
  },
  {
    name: "js-read-nested-struct-dead",
    breaks: "a struct in a field of one given a binding has its fields said never read",
    file: "src/lower/liveness.rs",
    find: "            structs.extend(local_structs(ty));\n",
    replace: "            let _ = ty;\n",
    tests: binding,
  },
  {
    name: "js-read-of-any-call",
    breaks: "a struct given any function, not only a binding, has its fields counted read, so a field nothing reads is never flagged",
    file: "src/lower/liveness.rs",
    find: "        if callee.is_some_and(|callee| super::bindings::is_binding(self.tcx, callee)) {\n",
    replace: "        if callee.is_some() {\n",
    tests: binding,
  },
  {
    name: "js-read-made-parts-unseen",
    breaks: "a struct given a binding in what's made of it, `Some(&options)`, has its fields said never read, as its value's type is coerced, an `Option<&dyn Any>`",
    file: "src/lower/liveness.rs",
    find: "        for part in parts {\n            self.gather(results, part);\n",
    replace: "        for part in parts {\n            let _ = part;\n",
    tests: binding,
  },
  {
    name: "js-class-dead",
    breaks: "a class only `js::class::<T>()` hands JS is said never constructed, its `new` never used and its fields never read",
    file: "src/lower/liveness.rs",
    find: "            self.classes.push(*adt);\n",
    replace: "",
    tests: clippy,
  },
  {
    name: "js-class-uses-unwalked",
    breaks: "what a class's `new` alone uses, a function and a struct it makes, is said never used",
    file: "src/lower/liveness.rs",
    find: "        if self.tcx.hir_maybe_body_owned_by(id).is_some() {\n",
    replace: "        if false {\n",
    tests: clippy,
  },
  {
    name: "js-class-fields-unread",
    breaks: "a class's field only JS reads, `this.listeners`, is said never read",
    file: "src/lower/liveness.rs",
    find: "    structs.extend(given.classes);\n",
    replace: "",
    tests: clippy,
  },
  {
    name: "js-class-paths-unread",
    breaks: "what a class's `new` names by a path, a function it calls or a tuple struct it makes, `Reason(\"none\")`, is said never used",
    file: "src/lower/liveness.rs",
    find: "            hir::ExprKind::Path(ref path) => self.mark_res(results.qpath_res(path, expr.hir_id)),\n",
    replace: "",
    tests: clippy,
  },
  {
    name: "js-class-methods-unseen",
    breaks: "a method a class's method calls, `self.signal.abort()`, is said never used",
    file: "src/lower/liveness.rs",
    find: "                if let Some(id) = results.type_dependent_def_id(expr.hir_id).and_then(|id| id.as_local()) {\n",
    replace: "                if let Some(id) = results.type_dependent_def_id(expr.hir_id).and_then(|id| id.as_local()).filter(|_| false) {\n",
    tests: clippy,
  },
  {
    name: "js-class-fields-read-unseen",
    breaks: "a field a class's method reads, `self.signal.aborted`, is said never read",
    file: "src/lower/liveness.rs",
    find: "                    self.mark(local);\n                }\n            }\n            _ => {}\n",
    replace: "                    let _ = local;\n                }\n            }\n            _ => {}\n",
    tests: clippy,
  },
];
