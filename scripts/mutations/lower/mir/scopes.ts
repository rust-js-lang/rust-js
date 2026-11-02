// Mutations of src/lower/mir/scopes.rs (ADR 1043).
import type { Mutation } from "../../../mutations";

const scopes = ["test/corpus.test.ts", "-t", "scope_struct"];

export const mutations: Mutation[] = [
  {
    name: "scope-functions-before-their-value",
    breaks: "a scope struct's methods' functions are declared before its value's `let`, not after it",
    file: "src/lower/mir/scopes.rs",
    find: "                list.splice(last + 1..last + 1, std::mem::take(&mut functions));",
    replace: "                list.splice(0..0, std::mem::take(&mut functions));",
    tests: scopes,
    snapshots: true,
  },
  {
    name: "scope-function-takes-self",
    breaks: "a scope struct's method's function takes the value it closes over as a parameter, which no call gives it",
    file: "src/lower/mir/scopes.rs",
    find: "                params.remove(0);\n",
    replace: "",
    tests: scopes,
  },
  {
    name: "scope-function-names-its-own",
    breaks: "a scope struct's method's function names its variables apart from none of its function's, a `counter` of its own hiding the one it closes over",
    file: "src/lower/mir/scopes.rs",
    find: "            names: self.names.clone(),\n",
    replace: "            names: Default::default(),\n",
    tests: scopes,
  },
  {
    name: "scope-fields-of-a-value-read-whole",
    breaks: "a scope struct's value its JS reads whole, `$fieldCell(tally, \"n\")`, is made variables of its fields, the object it reads gone",
    file: "src/lower/mir/scopes.rs",
    find: "    if whole || rebound || reads != fields_read + aliases || declared != holders.len() {",
    replace: "    if rebound || declared != holders.len() {",
    tests: scopes,
  },
  {
    name: "scope-field-written-a-const",
    breaks: "a scope struct's field its functions write is a `const`, which writing throws of",
    file: "src/lower/mir/scopes.rs",
    find: "                    match (written.contains(&key), e) {",
    replace: "                    match (false, e) {",
    tests: scopes,
  },
  {
    name: "scope-field-handle-unwritten",
    breaks: "a scope struct's field a handle writes, `&mut counter.n`, is a `const`",
    file: "src/lower/mir/scopes.rs",
    find: "        js::ExprKind::Handle(place, _) => place.each(&mut |e| {\n            if let js::ExprKind::Member(of, key) = &e.kind\n                && held(of)\n            {\n                written.insert(key.clone());\n            }\n        }),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_kept.rs$"],
  },
  {
    name: "scope-field-param-assigned",
    breaks: "a scope struct's field a parameter gives is that parameter where the function assigns it after, `step += 1`",
    file: "src/lower/mir/scopes.rs",
    find: "            js::ExprKind::Var(p) if param_names.contains(&p.as_str()) && !assigned(p) && !written.contains(&key) => {",
    replace: "            js::ExprKind::Var(p) if param_names.contains(&p.as_str()) && !written.contains(&key) => {",
    tests: scopes,
  },
  {
    name: "scope-field-param-written",
    breaks: "a scope struct's field a parameter gives and its functions write is that parameter, which the function reads after, changed",
    file: "src/lower/mir/scopes.rs",
    find: "            js::ExprKind::Var(p) if param_names.contains(&p.as_str()) && !assigned(p) && !written.contains(&key) => {",
    replace: "            js::ExprKind::Var(p) if param_names.contains(&p.as_str()) && !assigned(p) => {",
    tests: scopes,
  },
  {
    name: "scope-fields-named-unapart",
    breaks: "a scope struct's field's variable is named as its field where the function has a variable of that name, `parts`",
    file: "src/lower/mir/scopes.rs",
    find: "                let name = crate::names::fresh_in(&mut taken, &key);",
    replace: "                let name = key.clone();",
    tests: scopes,
  },
  {
    name: "scope-field-reads-kept",
    breaks: "a read of a scope struct's field stays its property's, of the object its variables replace",
    file: "src/lower/mir/scopes.rs",
    find: "            *e = Expr::var(&names[key]);",
    replace: "            let _ = &names;",
    tests: scopes,
  },
  {
    name: "scope-copies-kept",
    breaks:
      "a copy of a scope's field the program names as the field is kept, `const controller = controller$1;`, its variable named apart, where upstream's processReply reads `replyLifetimeController` (ADR 1043, amended)",
    file: "src/lower/mir/scopes.rs",
    find: "    js::each_block_mut(out, &mut |list| copies_read_through(list, &held));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^scope_field_copies.rs$"],
    snapshots: true,
  },
  {
    name: "scope-copies-past-calls",
    breaks:
      "a copy of a scope's field read after a call that sets the field again, `bump(); log.push(`was ${at}`)`, is read through the field, `99` where Rust's copy was `5` (ADR 1043, amended)",
    file: "src/lower/mir/scopes.rs",
    find: "                self.stopped |= e.has_effects();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^scope_field_copies.rs$"],
    snapshots: true,
  },
  {
    name: "scope-copies-past-writes",
    breaks:
      "a copy of a scope's field read after a statement whose order isn't followed, a write of the field, is read through it (ADR 1043, amended)",
    file: "src/lower/mir/scopes.rs",
    find: "                _ => self.stopped = true,\n",
    replace: "                _ => {}\n",
    tests: ["test/corpus.test.ts", "-t", "^scope_field_copies.rs$"],
    snapshots: true,
  },
  {
    name: "scope-copies-set-unfolded",
    breaks:
      "a field set to a copy the program names as it, `const controller = base * 3; controller$1 = controller;`, is kept apart, where upstream's is `replyLifetimeController = new AbortController();` (ADR 1043, amended)",
    file: "src/lower/mir/scopes.rs",
    find: "                Some((a.clone(), target.clone(), Some(e.clone())))\n",
    replace: "                None\n",
    tests: ["test/corpus.test.ts", "-t", "^scope_field_copies.rs$"],
    snapshots: true,
  },
  {
    name: "scope-copies-branches-in-turn",
    breaks:
      "an `if`'s branches are walked one after the other, its `else` after what its `then` does, so `if (controller != null) { controller.abort(..) }`'s copy is kept (ADR 1043, amended)",
    file: "src/lower/mir/scopes.rs",
    find: "                    (self.seen, self.stopped) = (seen, stopped);\n",
    replace: "                    self.stopped = stopped;\n",
    tests: ["test/corpus.test.ts", "-t", "^scope_field_copies.rs$"],
    snapshots: true,
  },
  {
    name: "scope-copies-after-branch-effects",
    breaks:
      "what's after an `if` whose branch calls what sets the field, `if (again) { bump(); } log.push(`maybe ${at}`)`, is taken for before it, `99` where Rust's copy was `5` (ADR 1043, amended)",
    file: "src/lower/mir/scopes.rs",
    find: "                    self.stopped |= then.1;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^scope_field_copies.rs$"],
    snapshots: true,
  },
  {
    name: "scope-copies-objects-unordered",
    breaks:
      "an object's fields aren't read in order, so `listen({ signal: controller + 1 })` keeps the copy, where upstream's `{signal: replyLifetimeController.signal}` reads the variable (ADR 1043, amended)",
    file: "src/lower/mir/scopes.rs",
    find: "                    .all(|p| matches!(p, js::Prop::Field(..) | js::Prop::Spread(_))) =>\n",
    replace: "                    .all(|_| false) =>\n",
    tests: ["test/corpus.test.ts", "-t", "^scope_field_copies.rs$"],
    snapshots: true,
  },
];
