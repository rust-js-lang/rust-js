// Mutations of src/lower/mir/closures.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "local-const-after-its-read",
    breaks:
      "a `const` written in a function's body is where it's written though a closure before that reads it, which JS's `const` isn't yet",
    file: "src/lower/mir/closures.rs",
    find: "                    written.zip(first).map(|(w, f)| w.min(f)).or(written).or(first)\n",
    replace: "                    written.or(first)\n",
    tests: ["test/corpus.test.ts", "-t", "^local_consts.rs$"],
  },
  {
    name: "local-const-after-a-call",
    breaks:
      "a `const` a function written in the body reads is where it's written, after a call of that function",
    file: "src/lower/mir/closures.rs",
    find: "                Some(_) if called => Some(0),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^local_consts.rs$"],
  },
  {
    name: "mir-async-closure-called-body",
    breaks:
      "an `async` closure is an arrow that calls an async arrow, not the async arrow",
    file: "src/lower/mir/closures.rs",
    find: "            && let Some(mut body) = called_async_body(&mut stmts)\n",
    replace:
      "            && let Some(mut body) = called_async_body(&mut stmts).filter(|_| false)\n",
    tests: ["test/compiler.test.ts", "-t", "async code becomes"],
  },
  {
    name: "mir-closure-parent-thir",
    breaks:
      "a closure's body is lowered with its parent's THIR, so its `if let` chain's tests stay nested",
    file: "src/lower/mir/closures.rs",
    find: "        let outer = std::mem::replace(&mut self.frame.thir, &body.thir);\n",
    replace: "        let outer = self.frame.thir;\n",
    tests: [
      "test/lowering-loops.test.ts",
      "-t",
      "if-let chains and loops from MIR",
    ],
  },
  {
    name: "mir-closure-parent-facts",
    breaks: "a closure's body reads its parent's facts, by another body's ids",
    file: "src/lower/mir/closures.rs",
    find: "        let outer_facts = std::mem::replace(&mut self.frame.facts, &body.facts);\n",
    replace: "        let outer_facts = self.frame.facts;\n",
    tests: ["test/lowering-iterators.test.ts", "-t", "string's parts from MIR"],
  },
  {
    name: "mir-spawning-closure-arrow",
    breaks:
      "a closure that only spawns an async block is `() => { (async () => …)(); }`, not `async () => …`",
    file: "src/lower/mir/closures.rs",
    find: "            && let js::ExprKind::AsyncArrow(inner, spawned) = &callee.kind\n            && inner.is_empty()\n        {\n            return Ok(Expr::async_arrow(params, spawned.clone()));\n",
    replace:
      "            && let js::ExprKind::AsyncArrow(inner, spawned) = &callee.kind\n            && inner.is_empty()\n            && false\n        {\n            return Ok(Expr::async_arrow(params, spawned.clone()));\n",
    tests: [
      "test/lowering-functions.test.ts",
      "-t",
      "only spawns an async block",
    ],
  },
  {
    name: "mir-capture-copied-always",
    breaks:
      "a closure copies a variable written only before it, `const captured = index`",
    file: "src/lower/mir/closures.rs",
    find: "                            && written_from(state.body, state.at, l)));\n",
    replace: "                            ));\n",
    tests: [
      "test/lowering-functions.test.ts",
      "-t",
      "captures a variable itself where nothing",
    ],
  },
  {
    name: "mir-closure-forwarding-unmarked",
    breaks:
      "a closure from MIR that passes on what it's given to Rust's function stays `(x) => double(x)`, not `double`",
    file: "src/lower/mir/closures.rs",
    find: "        let forwarded = self.forwarded_to(body, span);\n",
    replace:
      "        let forwarded = None::<crate::lower::passed_on::Forwarding>;\n",
    tests: [
      "test/lowering-functions.test.ts",
      "-t",
      "only passes its arguments on",
    ],
  },
  {
    name: "mir-closure-concise-unmarked",
    breaks:
      "a closure of one call that gives nothing isn't marked, so a held handler keeps its block",
    file: "src/lower/mir/closures.rs",
    find: "        let concise = self.closure_calls_for_nothing(body);\n",
    replace: "        let concise = None;\n",
    tests: [
      "test/jsx-children.test.ts",
      "test/jsx-evaluation-order.test.ts",
      "test/jsx-events.test.ts",
      "test/jsx-props.test.ts",
      "test/jsx-react-dev.test.ts",
      "test/jsx-types.test.ts",
      "test/jsx.test.ts",
    ],
  },
  {
    name: "mir-written-fn-in-closure-at-top",
    breaks:
      "a function written in a closure is declared at its function's top from MIR, not in the closure",
    file: "src/lower/mir/closures.rs",
    find: "            .filter(|&&id| self.tcx.parent(id) == def_id)",
    replace: "            .filter(|_| false)",
    tests: [
      "test/lowering-functions.test.ts",
      "-t",
      "a function written in a closure is declared where it's written",
    ],
  },
  {
    name: "mir-written-fn-before-all",
    breaks:
      "a function written in a body is declared before all of it from MIR, not before what's written after it",
    file: "src/lower/mir/closures.rs",
    find: ".position(|s| !s.span.is_none() && s.span.lo > at)",
    replace: ".position(|_| true)",
    tests: [
      "test/lowering-functions.test.ts",
      "-t",
      "a function written in a closure is declared where it's written",
    ],
  },
  {
    name: "mir-async-closure-wild-param-kept",
    breaks:
      "an `async` closure's `|_|` from MIR is `async (_) => ..`, not `async () => ..`",
    file: "src/lower/mir/closures.rs",
    find: '            && (name == "_" || name.starts_with("_$"))\n',
    replace: '            && name == "never"\n',
    tests: [
      "test/lowering-functions.test.ts",
      "-t",
      "an async closure's ignored argument is no parameter",
    ],
  },
  {
    name: "mir-mut-capture-copied",
    breaks:
      "a closure's `&mut` capture of its maker's variable isn't the variable, so `std::mem::take(&mut current)` in it empties the array in place, `current.splice(0)`, where the maker's own assigns it, `current = []`",
    file: "src/lower/mir/closures.rs",
    find: "                        upvars.get(i).map(|t| t.kind()),\n                        Some(ty::Ref(_, _, ty::Mutability::Mut))\n",
    replace:
      "                        upvars.get(i).map(|t| t.kind()).filter(|_| false),\n                        Some(ty::Ref(_, _, ty::Mutability::Mut))\n",
    tests: [
      "test/lowering-functions.test.ts",
      "-t",
      "taking a variable assigns the variable",
    ],
  },
  {
    name: "mir-closure-unmade",
    breaks:
      "MIR's closure holding a value with a destructor can't be dropped: where it's made isn't known",
    file: "src/lower/mir/closures.rs",
    find: "        self.closure_made(def_id, held);\n",
    replace: "        self.closure_made(def_id, Vec::new());\n",
    tests: ["test/corpus.test.ts", "-t", "closure_drops"],
  },
  {
    name: "mir-async-closure-directive-lost",
    breaks: "MIR's async closure loses its `js::directive!`",
    file: "src/lower/mir/closures.rs",
    find: "            body.splice(0..0, directives);\n",
    replace: "",
    tests: ["test/modules.test.ts", "-t", "js::directive! in a body"],
  },
  {
    name: "mir-closure-directive-lost",
    breaks: "MIR's closure loses its `js::directive!`",
    file: "src/lower/mir/closures.rs",
    find: "        stmts.splice(0..0, directives);\n",
    replace: "",
    tests: ["test/modules.test.ts", "-t", "js::directive! in a body"],
  },
  {
    name: "mir-spawning-closure-directive-lost",
    breaks: "MIR's closure that only spawns loses its `js::directive!`",
    file: "src/lower/mir/closures.rs",
    find: "            && directives.is_empty()\n",
    replace: "",
    tests: ["test/modules.test.ts", "-t", "js::directive! in a body"],
  },
  {
    name: "closure-owned-by-parent",
    breaks:
      "a closure's body is its parent's, so `Location::caller()` in it is the parent's caller's",
    file: "src/lower/mir/closures.rs",
    find: "        let outer_owner = std::mem::replace(&mut self.frame.owner, def_id);\n",
    replace: "        let outer_owner = self.frame.owner;\n",
    tests: ["test/corpus.test.ts", "-t", "^track_caller.rs$"],
  },
  {
    name: "returned-closure-unwrapped",
    breaks:
      "a closure its maker gives back carries no drop, which its caller's `$dispose` reads",
    file: "src/lower/mir/closures.rs",
    find: '        Ok(Expr::call_helper(\n            "$dropping",\n            vec![closure, Expr::arrow(Vec::new(), dropped)],\n        ))\n',
    replace: "        let _ = dropped;\n        Ok(closure)\n",
    tests: ["test/corpus.test.ts", "-t", "^returned_closure_drops.rs$"],
  },
  {
    name: "cloned-apart-captures-shared",
    breaks:
      "a closure cloned apart is made again of what it took, not a clone of it",
    file: "src/lower/mir/closures.rs",
    find: "            clones.push(self.clone_value(Expr::var(name), *ty, span, &mut cloned)?);\n",
    replace: "            clones.push(Expr::var(name));\n",
    tests: ["test/corpus.test.ts", "-t", "^closure_clone_cell.rs$"],
  },
  {
    name: "move-capture-of-reassigned-param-shared",
    breaks:
      "a `move` closure of a `mut` parameter assigned again after it's made reads the parameter, not its own copy, its argument not counted as a write",
    file: "src/lower/mir/closures.rs",
    find: "                            && super::queries::written_again(state, l)\n",
    replace: "                            && state.locals.writes[l] > 1\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_param_moved_out.rs$"],
  },
  {
    name: "written-fn-in-its-body-list",
    breaks:
      "a function written in a block is declared in its body's own list, not the innermost block that holds each use of it",
    file: "src/lower/mir/closures.rs",
    find: "        if let Some(written_in) = written_in {\n",
    replace:
      "        if let Some(written_in) = written_in.filter(|_| false) {\n",
    tests: ["test/js-object.test.ts", "-t", "a constructor is a fn item"],
  },
  {
    name: "written-fn-in-the-block-using-it",
    breaks:
      "a function written at a body's top is declared in the innermost block that uses it, an `if`'s, not the block it's written in",
    file: "src/lower/mir/closures.rs",
    find: "            if written_in.is_none()\n",
    replace: "            if true\n",
    tests: [
      "test/lowering-functions.test.ts",
      "-t",
      "a function written in a body is declared in it",
    ],
  },
  {
    name: "closure-result-held-untold",
    breaks:
      "a closure whose maker holds its result as the value isn't told so, and is refused, its body boxing it (rd547, ADR 1017 amended)",
    file: "src/lower/mir/closures.rs",
    find: "        let result_held = state.facts.options().held_results.contains(&def_id);\n",
    replace: "        let result_held = false;\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "with_borrow gives back"],
  },
  {
    name: "getter-capture-read-first",
    breaks: "a closure's shared borrow of a field of a struct JS made is read where the closure is made, `const captured = entry.value; $catchUnwind(() => stringify(captured))`, not where it's derefed (rd631)",
    file: "src/lower/mir/closures.rs",
    find: "                        && super::queries::derefs_capture_only(&mir.body, i) =>\n",
    replace: "                        && false =>\n",
    tests: ["test/lowering-moves.test.ts", "-t", "a closure's borrow of a getter's field"],
  },
  {
    name: "getter-capture-given-on-read-late",
    breaks: "a closure that gives its borrow of a getter's field on, `kept(&entry.value)`, reads it where it's called, not where it's made (rd631)",
    file: "src/lower/mir/closures.rs",
    find: "                        && super::queries::derefs_capture_only(&mir.body, i) =>\n",
    replace: "                        && true =>\n",
    tests: ["test/lowering-moves.test.ts", "-t", "a closure's borrow of a getter's field"],
  },
  {
    name: "capture-named-captured",
    breaks: "a closure's by-value capture of a variable, read first, is `const captured = descriptor.value;`, not named for the variable, `originalMethod` (rd656)",
    file: "src/lower/mir/closures.rs",
    find: "                        let base = (self.whole_capture_name(def_id, i)).filter(|name| {\n",
    replace: "                        let base = (None::<String>).filter(|name| {\n",
    tests: ["test/lowering-moves.test.ts", "-t", "a closure's capture read first is named for its variable"],
  },
  {
    name: "capture-copy-named-apart",
    breaks: "a closure's copy of a variable as it is now, which it changes after, is named for the variable apart, `const n$1 = n;` of `let mut n = n;`, not `captured` (rd656)",
    file: "src/lower/mir/closures.rs",
    find: "                                    && crate::lower::camel_case(info.name.as_str()) == *name))\n",
    replace: "                                    && false))\n",
    tests: ["test/lowering-moves.test.ts", "-t", "a closure's capture read first is named for its variable"],
  },
  {
    name: "capture-part-named-for-its-variable",
    breaks: "a closure's capture of a part of a variable, `entry.value`, is named for the variable alone, `const entry$1 = entry.value;` (rd656)",
    file: "src/lower/mir/closures.rs",
    find: "        (captured.place.projections.is_empty() && !name.as_str().is_empty())\n",
    replace: "        (!name.as_str().is_empty())\n",
    tests: ["test/lowering-moves.test.ts", "-t", "a closure's capture read first is named for its variable"],
  },
];
