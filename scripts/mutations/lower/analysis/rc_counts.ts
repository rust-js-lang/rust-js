// Mutations of src/lower/analysis/rc_counts.rs (ADR 0320).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "library-counts-every-rc",
    breaks: "a library that reads no count counts every `Rc`, React's context `{ value, strong, weak }` (ADR 0844)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "    counted.all |= shared.library && counted.any();\n",
    replace: "    counted.all |= shared.library;\n",
    tests: ["test/crates.test.ts", "-t", "reads no count"],
  },
  {
    name: "library-counts-some-rcs",
    breaks: "a library counts the `Rc`s it reads a count of alone, which its consumers share uncounted (ADR 0844)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "    counted.all |= shared.library && counted.any();\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "three crates"],
  },
  {
    name: "copy-param-dropping",
    breaks: "an `Rc` of a `T: Copy`'s value is taken to drop it, so a library holding one counts every `Rc` (ADR 0844)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "            Some(t) if known_by_bounds(t) && t.needs_drop(tcx, env) => return true,\n",
    replace: "            Some(t) if known_by_bounds(t) => return true,\n",
    tests: ["test/crates.test.ts", "-t", "reads no count"],
  },
  {
    name: "held-fields-unseen",
    breaks: "a generic `Rc` in a library's struct a body drops isn't seen, so it counts none, and its consumer, given a value with a destructor, is refused (ADR 0844)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "                if env.is_some()\n                    && let ty::Adt(adt, args) = *part.kind()\n",
    replace: "                if false\n                    && let ty::Adt(adt, args) = *part.kind()\n",
    tests: ["test/crates.test.ts", "-t", "reads no count"],
  },
  {
    name: "rc-generic-uncounted",
    breaks: "a generic function's `Rc<T>` isn't counted where its caller's is",
    file: "src/lower/analysis/rc_counts.rs",
    find: "        || !counted.types.is_empty() && generic_rc\n",
    replace: "        || false && generic_rc\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_count_through_generic.rs$"],
  },
  {
    name: "rc-destructor-uncounted",
    breaks: "an `Rc` of a value with a destructor isn't counted, and is refused",
    file: "src/lower/analysis/rc_counts.rs",
    find: "            .filter(|&&pointee| !counted.types.contains(&pointee) && runs(&counted, pointee))\n",
    replace: "            .filter(|&&pointee| !counted.types.contains(&pointee) && false && runs(&counted, pointee))\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_destructors.rs$"],
  },
  {
    name: "rc-destructor-one-round",
    breaks: "an `Rc` of a value that holds a counted `Rc` isn't counted, as its drop runs only once that one is",
    file: "src/lower/analysis/rc_counts.rs",
    find: "        counted.types.extend(dropping);\n    }\n",
    replace: "        counted.types.extend(dropping);\n        break;\n    }\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_destructors.rs$"],
  },
  {
    "name": "rc-made-counted-uncounted",
    "breaks": "an `Rc` made a counted `Rc<RefCell<dyn T>>` isn't counted itself: the two disagree on what an `Rc` is, and a method reads what isn't there",
    "file": "src/lower/analysis/rc_counts.rs",
    "find": "                .filter(|(from, to)| counted.types.contains(to) && !counted.types.contains(from))\n",
    "replace": "                .filter(|_| false)\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "rc_dyn_sinks"
    ]
  },
  {
    name: "generic-rc-uncounted",
    breaks: "an `Rc` of a type parameter's value, in a crate with a destructor, isn't counted, and its drop is refused",
    file: "src/lower/analysis/rc_counts.rs",
    find: "        || params_may_drop && holds_param;\n",
    replace: ";\n",
    tests: ["test/corpus.test.ts", "-t", "^(generic_rc_drops|rc_of_param_unnamed).rs$"],
  },
  {
    name: "generic-rc-of-reference-counted",
    breaks: "an `Rc` of a reference to a type parameter's value is counted, though it drops nothing it points at",
    file: "src/lower/analysis/rc_counts.rs",
    find: "            Some(t) if matches!(t.kind(), ty::Ref(..)) => walk.skip_current_subtree(),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^rc_of_borrowed_param.rs$"],
    snapshots: true,
  },
  {
    name: "rc-count-fn-value-unread",
    breaks: "a count read by a function given as a value, `.map(Rc::strong_count)`, doesn't count its `Rc`s, and the compiler panics",
    file: "src/lower/analysis/rc_counts.rs",
    find: "            if let Some((def_id, args)) = fn_def(expr.ty)\n",
    replace: "            if let Some((def_id, args)) = fn_def(expr.ty).filter(|_| matches!(expr.kind, ExprKind::Call { .. }))\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_count_fn_value.rs$"],
  },
];
