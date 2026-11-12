// Mutations of src/lower/analysis/rc_counts.rs (ADR 0320).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "lent-closure-unseen",
    breaks: "a closure a reference to which is made a `&dyn Fn` isn't one whose copies are apart, so `ptr::eq(&c1, &c2)` of a copy is `true` where native Rust's is `false` (#1090)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "                    ty::Closure(..) => _ = lent.insert(erased(from)),\n",
    replace: "                    ty::Closure(..) => {}\n",
    tests: ["test/corpus.test.ts", "-t", "^copied_closures_compared.rs$"],
  },
  {
    name: "lent-param-unseen",
    breaks: "a type parameter's value lent as a `&dyn Fn` by generic code isn't taken for any closure's, so a copy of one given it compares equal to the original (#1090)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "                    ty::Param(..) | ty::Alias(..) => lent_any = true,\n",
    replace: "                    ty::Param(..) | ty::Alias(..) => {}\n",
    tests: ["test/corpus.test.ts", "-t", "^copied_closures_lent_to_generic_code.rs$"],
  },
  {
    name: "lent-of-a-box",
    breaks: "a `Box` of a closure made a `dyn Fn` is taken for a reference lent, so its copies are `.bind()` before each is made a function of its own anyway (#1090)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "                && (from_ty.is_ref() || from_ty.is_raw_ptr())\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_addresses_of_boxes.rs$"],
    snapshots: true,
  },
  {
    name: "self-handle-upgrade-counts",
    breaks:
      "a closure's upgrade of its handle on itself counts its `Rc`, `{ value, strong, weak }`, where it always gives that `Rc` (ADR 0843, amended)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "                    && !self_handles.asks_no_count(body.def_id, expr.span)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^self_handle.rs$"],
    snapshots: true,
  },
  {
    name: "copy-param-dropping",
    breaks:
      "an `Rc` of a `Copy` associated type's value, `U::Value`'s, is taken to drop it, so a library holding one counts it (ADR 0844)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "                Some(t) if known_by_bounds(t) && t.needs_drop(tcx, env) => return true,\n",
    replace: "                Some(t) if known_by_bounds(t) => return true,\n",
    tests: ["test/crates.test.ts", "-t", "reads no count"],
  },
  {
    name: "held-fields-unseen",
    breaks:
      "a generic `Rc` in a library's struct a body drops isn't seen, so it counts none, and its consumer, given a value with a destructor, is refused (ADR 0844)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "            } else if let ty::Adt(adt, args) = *part.kind()\n                && !is_std_crate(tcx, adt.did())\n",
    replace:
      "            } else if false\n                && let ty::Adt(adt, args) = *part.kind()\n                && !is_std_crate(tcx, adt.did())\n",
    tests: ["test/crates.test.ts", "-t", "reads no count"],
  },
  {
    name: "borrowed-place-owned",
    breaks:
      "a generic function reading a struct through a reference, `shelf.items.len()`, is taken to own its `Rc<Item<T>>`s, which it may drop, so it counts them (ADR 0852)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "        ExprKind::Deref { .. } => true,\n",
    replace: "        ExprKind::Deref { .. } => false,\n",
    tests: ["test/crates.test.ts", "-t", "counts each type"],
  },
  {
    name: "fn-item-owned",
    breaks:
      "a function's type, `Vec::<Rc<Item<T>>>::len`'s, is taken to own its arguments' types' values, so a generic function reading a shelf counts its `Rc<Item<T>>`s (ADR 0852)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "            if matches!(part.kind(), ty::Ref(..) | ty::FnDef(..))\n",
    replace: "            if matches!(part.kind(), ty::Ref(..))\n",
    tests: ["test/crates.test.ts", "-t", "counts each type"],
  },
  {
    name: "weak-owns-pointee",
    breaks:
      "a `Weak` is taken to own what it points at, so a generic function holding React's context's consumer, whose `_context` is one, counts it (ADR 0852)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "                Some(t) if matches!(t.kind(), ty::Ref(..)) || weak_pointee(tcx, t).is_some() => {\n",
    replace:
      "                Some(t) if matches!(t.kind(), ty::Ref(..)) => {\n",
    tests: ["test/crates.test.ts", "-t", "counts each type"],
  },
  {
    name: "rc-made-counted-uncounted",
    breaks:
      "an `Rc` made a counted `Rc<RefCell<dyn T>>` isn't counted itself: the two disagree on what an `Rc` is, and a method reads what isn't there",
    file: "src/lower/analysis/rc_counts.rs",
    find: "                .filter(|(from, to)| counted.types.contains(to) && !counted.types.contains(from))\n",
    replace: "                .filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "rc_dyn_sinks"],
  },
  {
    name: "generic-rc-uncounted",
    breaks:
      "an `Rc` of a type parameter's value, in a crate with a destructor, isn't counted, and its drop is refused",
    file: "src/lower/analysis/rc_counts.rs",
    find: "                counted.insert(tcx.erase_and_anonymize_regions(pointee));\n",
    replace: "                let _ = pointee;\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(generic_rc_drops|rc_of_param_unnamed|rc_dropped_where_callers_unseen).rs$",
    ],
  },
  {
    name: "generic-rc-of-reference-counted",
    breaks:
      "an `Rc` of a reference to a type parameter's value is counted, though it drops nothing it points at",
    file: "src/lower/analysis/rc_counts.rs",
    find: "                Some(t) if matches!(t.kind(), ty::Ref(..)) || weak_pointee(tcx, t).is_some() => {\n",
    replace: "                Some(t) if weak_pointee(tcx, t).is_some() => {\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_of_borrowed_param.rs$"],
    snapshots: true,
  },
  {
    name: "may-be-ignores-arguments",
    breaks:
      "two types of one constructor are taken to be one whatever they're of, so counting an `Rc<Cell<f64>>` counts every `Rc<Cell<_>>` (ADR 0852)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "            a == b && of_a.len() == of_b.len() && of_a.iter().zip(&of_b).all(|(&a, &b)| may_be(tcx, a, b))\n",
    replace: "            a == b\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_counted_per_type.rs$"],
    snapshots: true,
  },
  {
    name: "rc-count-fn-value-unread",
    breaks:
      "a count read by a function given as a value, `.map(Rc::strong_count)`, doesn't count its `Rc`s, and the compiler panics",
    file: "src/lower/analysis/rc_counts.rs",
    find: "            if let Some((def_id, args)) = fn_def(expr.ty) {\n",
    replace:
      "            if let Some((def_id, args)) = fn_def(expr.ty).filter(|_| matches!(expr.kind, ExprKind::Call { .. })) {\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_count_fn_value.rs$"],
  },
  {
    name: "generic-rc-of-any-param-counted",
    breaks:
      "a program's `Rc` of a value of a type parameter is counted where its bounds let it have a destructor, whatever its callers give, so the reconciler's `Rc<ReactContext<T>>` is `{ value, strong, weak }` (ADR 0914, amended)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "                    if given(param.index) && t.needs_drop(tcx, env) {\n",
    replace: "                    if t.needs_drop(tcx, env) {\n",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "judged by what its callers give",
    ],
  },
  {
    name: "fn-identity-of-fn-items",
    breaks:
      "an `Rc<dyn Fn>` made of a function item, or a type parameter's function, is compared uncounted, so two `Rc::new(hello)`s are `hello === hello`, one, where Rust's are apart (ADR 0959)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "        ty::Dynamic(predicates, ..) => is_fn_dyn(tcx, predicates),\n        _ => false,\n",
    replace:
      "        ty::Dynamic(predicates, ..) => is_fn_dyn(tcx, predicates),\n        _ => true,\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_fn_ptr_eq_of_(item|param).rs$"],
  },
  {
    name: "fn-identity-of-copied-closure",
    breaks:
      "an `Rc<dyn Fn>` made of a closure given `Rc::new` as a variable, a copy, is compared uncounted, so two of one `Copy` closure are one function (ADR 0959)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "            id.is_local()\n                && (made_with.contains(&from) && !made_apart.contains(&from)\n",
    replace:
      "            true || id.is_local()\n                && (made_with.contains(&from) && !made_apart.contains(&from)\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_fn_ptr_eq_of_copy.rs$"],
  },
  {
    name: "fn-identity-variable-made-where-given",
    breaks:
      "a closure given `Rc::new` by a variable is taken to be made there, so two `Rc`s of one `Copy` closure compare as one (ADR 0959)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "    matches!(thir[given].kind, ExprKind::Closure(_))\n",
    replace:
      "    matches!(thir[given].kind, ExprKind::Closure(_) | ExprKind::VarRef { .. })\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_fn_ptr_eq_of_copy.rs$"],
  },
  {
    name: "fn-identity-through-dyn-counted",
    breaks:
      "an `Rc<dyn Fn>` made another's, its lifetime's as it's pushed, is taken to be made of a function made apart, so React's commit hooks are counted, `{ value, strong, weak }` (ADR 0959)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "        ty::Dynamic(predicates, ..) => is_fn_dyn(tcx, predicates),\n",
    replace: "        ty::Dynamic(..) => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_fn_ptr_eq.rs$"],
    snapshots: true,
  },
  {
    name: "rc-fn-moved-made-apart",
    breaks:
      "an `Rc<dyn Fn>` made of a closure a `let` holds, neither `Copy` nor cloned, is counted where one is compared by `ptr_eq`, so React's Hooks, giving one to JS, is refused (ts7, ADR 0959, amended)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "                    || !copied.contains(&from) && !is_cloned(from))\n",
    replace: "                    || false)\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_fn_ptr_eq_of_moved.rs$"],
    snapshots: true,
  },
  {
    name: "rc-fn-cloned-uncounted",
    breaks:
      "an `Rc<dyn Fn>` made of a cloned closure, `Rc::new(read.clone())` beside `Rc::new(read)`, is the function, one JS function twice, so `ptr_eq` of the two is `true` (a wrong answer, ADR 0959, amended)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "    let is_cloned = |from: Ty<'tcx>| cloned.of_named_types().any(|clone| may_be(tcx, clone, from));\n",
    replace: "    let is_cloned = |_: Ty<'tcx>| false;\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_fn_ptr_eq_of_cloned.rs$"],
  },
  {
    name: "rc-fn-copied-coerced-uncounted",
    breaks:
      "an `Rc<dyn Fn>` made of a `Copy` closure given `Rc::new` by its variable twice is the function, so `ptr_eq` of the two is `true` (a wrong answer, ADR 0959, amended)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "\n                    copied.insert(erased(from));\n",
    replace: "\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_fn_ptr_eq_of_copy.rs$"],
  },
  {
    name: "rc-of-bound-function-made-apart",
    breaks:
      "an `Rc` of a `&dyn Fn` made of a function made with it, `Rc::from(function::bind(..))`, is one that may share it, so React's hooks' `dyn Fn`s are counted and drop, which `react_isomorphic`'s refuse (rd231)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "            made_with.contains(&from) && !made_apart.contains(&from)\n        }\n        ty::Closure(id, _) => {",
    replace: "            false\n        }\n        ty::Closure(id, _) => {",
    tests: ["test/cross-crate-dyn-fns.test.ts", "-t", "bound function, compared"],
  },
  {
    name: "rc-of-shared-reference-made-with",
    breaks:
      "an `Rc` of a `&dyn Fn` made of a reference given it is that function, so two of one reference are `ptr_eq` in JS where Rust's are two (rd231)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "                        false => made_apart.insert(erased(from)),\n                    };\n                }",
    replace:
      "                        false => made_with.insert(erased(from)),\n                    };\n                }",
    tests: ["test/cross-crate-dyn-fns.test.ts", "-t", "bound function, compared"],
  },
  {
    name: "rc-made-of-counted-uncounted",
    breaks:
      "an `Rc<dyn Fn>` made of a counted `Rc<{closure}>` isn't counted, so a library's caller calls the `{ value, strong, weak }` it is, `TypeError: callback is not a function` (#458, ADR 0852, amended)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "                .filter(|(from, to)| counted.types.contains(from) && !counted.types.contains(to))\n",
    replace: "                .filter(|(from, to)| false && counted.types.contains(from) && !counted.types.contains(to))\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_fn_made_of_a_counted_rc.rs$"],
  },
  {
    name: "rc-of-closure-holds-its-generics",
    breaks:
      "an `Rc` of a generic function's closure that captures nothing, React's decodeBoundActionMetaData<M>'s `Rc::new(|_| {})`, is taken to own a value of `M`, which its type names, so it's counted, and the `Rc<dyn Fn>`s it's made, and the closures that hold one drop, which react_isomorphic's `dyn Fn`s refuse (rd597, ADR 0626, amended)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "                Some(t) if let ty::Closure(_, args) = *t.kind() => {\n",
    replace: "                Some(t) if false && let ty::Closure(_, args) = *t.kind() => {\n",
    tests: ["test/cross-crate-dyn-fns.test.ts", "-t", "holds none of its type parameters"],
  },
  {
    name: "rc-of-closure-holds-nothing",
    breaks:
      "an `Rc` of a generic function's closure that captures a `T` given one that drops, or a `Box<T>`, isn't counted, so its first drop drops what it captured, before Rust's last does (rd597, ADR 0626, amended)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "                    held.extend(args.as_closure().upvar_tys());\n",
    replace: "                    let _ = args;\n",
    tests: ["test/cross-crate-dyn-fns.test.ts", "test/corpus.test.ts", "-t", "holds none of its type parameters|^rc_dropped_in_a_closure_where_callers_unseen.rs$"],
  },
  {
    name: "compared-uncounted-always",
    breaks:
      "a crate that compares no uncounted `Rc<dyn Fn>` is taken to compare one, so `Rc::new(move |a, b| renderer.discrete_updates(a, b))` stays an arrow, not `renderer.discreteUpdates` (rd313)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "    counted.compared_uncounted = !counted.all && asked.iter().any(|pointee| !counted.types.contains(pointee));\n",
    replace: "    counted.compared_uncounted = true;\n",
    tests: ["test/namespaces.test.ts", "-t", "through a value is that function"],
  },
  {
    name: "compared-uncounted-never",
    breaks:
      "a crate that compares uncounted `Rc<dyn Fn>`s is taken to compare none, so `Rc::new(|| ping())` twice is `ping` twice, and `Rc::ptr_eq` of the two is true where native gives false (rd313, ADR 0959)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "    counted.compared_uncounted = !counted.all && asked.iter().any(|pointee| !counted.types.contains(pointee));\n",
    replace: "    counted.compared_uncounted = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_fn_ptr_eq_of_forwarding.rs$"],
  },
  {
    name: "functions-pointed-at-unseen",
    breaks:
      "a crate that holds a raw pointer to a `dyn Fn`, `ptr::addr_eq(&*first, &*second)`'s, is taken to read no closure's identity, so two closures of one scope method are one, and the comparison true where native gives false (#500)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "                Some(IdentityRead::Pointed(pointee)) => {\n                    points_at_functions = true;\n",
    replace: "                Some(IdentityRead::Pointed(pointee)) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^scope_method_closure_addr_eq.rs$"],
  },
  {
    name: "impl-rc-pointees-unread",
    breaks:
      "an `Rc<T>` only an impl's header names, `impl<T> StableDeref for Rc<T>`, isn't a pointee, so beside a counted `Rc<dyn Fn>` its dictionary reads `.value` of an `Rc<Vec<u32>>` made uncounted: a `TypeError` (#949)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "            DefKind::Impl { .. } => found(\n",
    replace: "            DefKind::Impl { .. } if false => found(\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_of_an_impls_parameter_counted_as_any.rs$"],
  },
  {
    name: "rc-dyn-fn-any-other",
    breaks: "an `Rc<dyn Fn()>` may be any other `dyn Fn`'s, so a neighbour's `ptr_eq` of an `Rc<dyn Fn(u32) -> u32>` counts it, and a closure holding it drops (#1003)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "            let mut args: Vec<Ty<'tcx>> = super::closures::fn_dyn_args(tcx, predicates).into_iter().collect();\n            args.extend(\n",
    replace: "            let mut args: Vec<Ty<'tcx>> = Vec::new();\n            args.truncate(0);\n            Vec::<Ty<'tcx>>::new().extend(\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_dyn_fn_counted_by_its_signature.rs$"],
  },
  {
    name: "rc-dyn-fn-output-uncompared",
    breaks:
      "an `Rc<dyn Fn()>` may be an `Rc<dyn Fn() -> u32>`, as their arguments are one, so a consumer that counts the second, of a library's function it compiles, is refused the first a library hands it uncounted (ADR 0852, #1056)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "            args.extend(\n                (predicates.projection_bounds())\n",
    replace: "            args.extend(\n                (predicates.projection_bounds().filter(|_| false))\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_fn_dyns_of_two_results"],
  },
  {
    name: "crate-reads-no-fn-identity",
    breaks:
      "a consumer that compares a library's `Rc<dyn Fn>`s or its boxes' addresses is taken to compare none, and calls its library's JS, which gives one function where Rust made two: `Rc::ptr_eq` and `ptr::addr_eq` `true` (#1052)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "        body.thir.exprs.iter().any(|expr| identity_read(tcx, expr.ty).is_some())\n",
    replace: "        false && body.thir.exprs.iter().any(|expr| identity_read(tcx, expr.ty).is_some())\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_identities_kept_apart"],
  },
  {
    name: "uncounted-rc-of-a-clone-merges-nothing",
    breaks:
      "a library's `Rc<dyn Fn>`s of a cloned closure it doesn't count aren't listed, so its consumer calls its `cloned`, `const first = read; const second = read;`, and `Rc::ptr_eq` of the two is `true` (#1052)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "        .filter(|&&(_, from)| !its_own(from))\n        .map(|&(owner, _)| owner)\n",
    replace: "        .filter(|&&(_, from)| false && !its_own(from))\n        .map(|&(owner, _)| owner)\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_identities_kept_apart"],
  },
  {
    name: "rcs-an-instance-makes-unseen",
    breaks:
      "a library's function the crate compiles itself makes its `Rc<dyn Fn>` of a cloned closure, which the crate's `Rc::ptr_eq` compares, but the crate counts none of it, so both are `read` (#1052)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "                if let (Some(from), Some(to)) = (rc_pointee(tcx, from_owner), rc_pointee(tcx, to_owner)) {\n",
    replace:
      "                if let (Some(from), Some(to)) = (rc_pointee(tcx, from_owner), rc_pointee(tcx, to_owner))\n                    && false\n                {\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_identities_kept_apart"],
  },
  {
    name: "allocations-an-instance-makes-unseen",
    breaks:
      "a library's function the crate compiles itself, as it reads a `dyn Fn`'s address, boxes a `fn` pointer it's given, the pointer itself, so two boxes of one are `===` (#1053)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "                    unsized_fns.insert((erased(from_owner), erased(from), erased(to)));\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "two crates: library_allocations_of_one_function"],
  },
  {
    name: "library-allocations-merge-unlisted",
    breaks:
      "a library function that boxes a `fn` pointer it's given isn't listed as merging identities, so a consumer that reads a `dyn Fn`'s address calls its JS, the pointer itself, and two boxes of one are `===` (#1053)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "\n                allocated_by.push((owner, erased(from)));\n",
    replace: "\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_allocations_of_one_function"],
  },
  {
    name: "allocations-of-one-function-merged",
    breaks:
      "two `Box<dyn Fn>`s or `Rc<dyn Fn>`s made of one `fn` pointer, item or cloned closure are one JS function, so `ptr::addr_eq` of them is `true` where Rust's two allocations are apart (#1053)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "        .filter(|&&(_, from, to)| address_read(to) && !its_own(from))\n",
    replace: "        .filter(|&&(_, from, to)| false && address_read(to) && !its_own(from))\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_addresses_of_boxes.rs$"],
  },
  {
    name: "allocation-address-of-any-signature-read",
    breaks:
      "an `Rc<dyn Fn(&str) -> usize>` of a function item is made a function of its own, `(arg0) => words(arg0)`, where the crate reads the address of no `dyn Fn` of its arguments, React's `unsuspend: () => noop()` (#1053)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "    let address_read = |to: Ty<'tcx>| pointed.iter().any(|&read| may_be(tcx, read, to));\n",
    replace: "    let address_read = |_to: Ty<'tcx>| !pointed.is_empty();\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_addresses_of_rcs.rs$"],
    snapshots: true,
  },
  {
    name: "library-closure-moved-once-apart",
    breaks:
      "a closure a library's function boxes as it's made, compiled by a consumer that reads a `dyn Fn`'s address, is made a function of its own again, `const f = (previous) => ..; return { compute: (arg0) => f(arg0) }` (#1053)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "        |from: Ty<'tcx>| made_with_it(from) || from.is_closure() && !copied.contains(&from) && !is_cloned(from);\n",
    replace: "        |from: Ty<'tcx>| made_with_it(from);\n",
    tests: ["test/crates-compiled-here.test.ts", "-t", "merge what its consumer compares"],
  },
  {
    name: "rc-counts-clones-as-generic-code-names-them",
    breaks:
      "a closure generic code makes and clones, `shown::<u8>`'s, is asked as generic code names it of the clones' owner, which holds its callers' instances, so it's taken to be made with each `Rc::new`, the two `Rc`s one function, `ptr_eq` `true` (#1249)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "    let is_cloned = |from: Ty<'tcx>| cloned.of_named_types().any(|clone| may_be(tcx, clone, from));\n",
    replace: "    let is_cloned = |from: Ty<'tcx>| cloned.of_named_types().any(|clone| clone == from);\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_fn_ptr_eq_of_a_closure_cloned_in_generic_code.rs$"],
  },
  {
    name: "library-closure-copies-unseen",
    breaks:
      "a library's `Copy` closure given `Rc::new` twice, compiled by a consumer that reads a `dyn Fn`'s address, is taken to be made with each, so the two `Rc`s are one function, `addr_eq` `true` (#1053)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "                    if tcx.type_is_copy_modulo_regions(env, from) {\n                        copied.insert(erased(from));\n                    }\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "two crates: library_allocations_of_one_function"],
  },
  {
    name: "allocation-of-a-function-made-with-it-apart",
    breaks:
      "a closure written in its `Box::new` is made a function of its own again where it's unsized, `(arg0) => ..` around an arrow no other box holds (#1053)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "        .filter(|&&(_, from, to)| address_read(to) && !its_own(from))\n",
    replace: "        .filter(|&&(_, _from, to)| address_read(to))\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_addresses_of_boxes.rs$"],
    snapshots: true,
  },
  {
    name: "allocation-functions-apart-unread",
    breaks:
      "every `Rc<dyn Fn>` made of a function item is a function of its own, `(arg0) => hello(arg0)`, in a crate that reads no `dyn Fn`'s address (#1053)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "        .filter(|&&(_, from, to)| address_read(to) && !its_own(from))\n",
    replace: "        .filter(|&&(_, from, _to)| !its_own(from))\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_fn_ptr_eq_of_item.rs$"],
    snapshots: true,
  },
  {
    name: "box-new-closure-made-apart",
    breaks:
      "a `Copy` closure written in its `Box::new` isn't seen made there, so it's made a function of its own again where it's unsized (#1053)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "                    .is_some_and(|(def_id, _)| is_rc_new(tcx, def_id) || is_std_def(tcx, def_id, StdItem::BoxNew))\n",
    replace: "                    .is_some_and(|(def_id, _)| is_rc_new(tcx, def_id))\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_addresses_of_boxes.rs$"],
    snapshots: true,
  },
  {
    name: "rc-closures-apart-where-unsized",
    breaks:
      "an `Rc`'s closure is made a function of its own where it's unsized, not where it's given `Rc::new`, so two clones of one `Rc` made before are two functions, `ptr::addr_eq` `false` where Rust's is `true` (#1053)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "        .partition(|&&(owner, from)| !owner.is_box() && from.is_closure());\n",
    replace: "        .partition(|&&(owner, from)| false && !owner.is_box() && from.is_closure());\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_addresses_of_rcs.rs$"],
  },
  {
    name: "shared-rc-of-a-fn-pointer-unrefused",
    breaks:
      "an `Rc` of a `fn` pointer cloned before it's unsized is made two functions of its own, `ptr::addr_eq` `false` where Rust's is `true`, not refused (#1053)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "                && (cloned.with_compiled_here().any(|clone| may_be(tcx, clone, owner))\n",
    replace: "                && (cloned.with_compiled_here().any(|clone| false && may_be(tcx, clone, owner))\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_address_of_a_shared_rc_refused.rs$"],
  },
  {
    name: "upgraded-rc-of-a-fn-pointer-unrefused",
    breaks:
      "an `Rc` of a `fn` pointer and the one its `Weak` upgrades to, each unsized, are two functions of their own, their addresses apart where Rust's are one, not refused (#1053)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "                    || weaks.iter().any(|&weak| may_be(tcx, weak, from)))\n",
    replace: "                    || weaks.iter().any(|&weak| false && may_be(tcx, weak, from)))\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_address_of_an_upgraded_rc_refused.rs$"],
  },
  {
    name: "rc-made-where-unsized-shared",
    breaks:
      "a generic `Rc::new(f)` unsized where it's made is taken to be shared before, as the crate clones another `Rc`, and refused (#1053)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "                && unsized_later.contains(&owner)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_addresses_of_rcs.rs$"],
  },
  {
    name: "library-counts-every-rc",
    breaks: "a crate that counts one type's `Rc`s counts every one, React's context `{ value, strong, weak }` beside a counted `Rc<Slot<T>>` (ADR 0852)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "    counted.all = counted.types.iter().any(|&t| wild(t) && head(tcx, t).is_none());\n",
    replace: "    counted.all = !counted.types.is_empty();\n",
    tests: ["test/crates.test.ts", "-t", "counts each type"],
  },
  {
    name: "rc-generic-uncounted",
    breaks: "a generic function's `Rc<T>` isn't counted where its caller's is",
    file: "src/lower/analysis/rc_counts.rs",
    find: "            let may = (counted.types.iter()).any(|&t| (wild(t) || wild(pointee)) && may_be(tcx, t, pointee));\n",
    replace: "            let may = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^(rc_count_through_generic|rc_counted_per_type|rc_counted_where_callers_unseen).rs$"],
  },
  {
    name: "rc-destructor-uncounted",
    breaks: "an `Rc` of a value with a destructor isn't counted, and is refused",
    file: "src/lower/analysis/rc_counts.rs",
    find: "            if of_counted || instance || may || !pointee.has_non_region_param() && runs(&counted, pointee) {\n",
    replace: "            if of_counted || instance || may || false && runs(&counted, pointee) {\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_destructors.rs$"],
  },
  {
    name: "rc-destructor-one-round",
    breaks: "an `Rc` of a value that holds a counted `Rc` isn't counted, as its drop runs only once that one is",
    file: "src/lower/analysis/rc_counts.rs",
    find: "        counted.types.extend(more);\n    }\n",
    replace: "        counted.types.extend(more);\n        break;\n    }\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_destructors.rs$"],
  },
  {
    name: "rc-param-resolution-unasked",
    breaks: "a generic function's counted `Rc<T>` or `Rc<Slot<T>>` counts every `Rc` it may be, though each of its callers is seen giving `T` a type, so an untagged enum's `Rc` variants, or a class constructor's `Rc<Self>`, beside it are refused (#1001, #1009)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "            (pointee, made())\n",
    replace: "            (pointee, made().filter(|_| false))\n",
    tests: ["test/corpus.test.ts", "-t", "^(rc_counted_by_a_generic_beside_a_class|rc_counted_by_callers_instances).rs$"],
  },
  {
    name: "rc-resolved-after-settling",
    breaks: "which generic pointees' instances are seen isn't known while settling, so `runs`, asking `counts` of a type whose drop drops its own `Rc`, takes a resolved `T` for any type, and counts it: an untagged enum's `Rc<Label>` beside a bare `Rc<T>` of `Loud` is refused (#1009, the neighbour shuffle's seed 57)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "    counted.resolved = (instances.iter())\n",
    replace: "    counted.resolved = IndexSet::default();\n    let _: IndexSet<Ty<'tcx>> = (instances.iter())\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_counted_by_callers_instances.rs$"],
  },
  {
    name: "seen-uncounted-may-be-any",
    breaks: "generic code's `Rc<F>` of an `F` whose callers give functions, none counted, is counted as any type `F` may be beside a counted `Rc<u8>`, and the `Rc<dyn Fn>` made of it, which callers call uncounted, is `{ value, strong, weak }`: `a is not a function` (#1227)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "            || !self.seen.contains(&pointee)",
    replace: "            || !self.resolved.contains(&pointee)",
    tests: ["test/corpus.test.ts", "-t", "^rc_of_functions_uncounted_beside_a_counted_one\\.rs$"],
  },
  {
    name: "instance-rcs-owned-unseen",
    breaks:
      "an `Rc` of a value a caller gives a drop for, which a library's function the consumer compiles itself makes and shares, isn't counted, so the consumer is refused \"a std type holding a value with a destructor, `std::rc::Rc<T>`\" (#1332)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "                    owned_dropping(tcx, (env, &given), decl.ty, &mut owned_seen, &mut counted);\n",
    replace: "                    _ = (&given, &owned_seen);\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_rc_shared_by_its_consumers_instance"],
  },
  {
    name: "instance-rc-counts-unread",
    breaks:
      "a count a library's function the consumer compiles itself reads, `Rc::strong_count`, isn't seen, so its `Rc` of a `Copy` value isn't counted where nothing else counts it (#1332)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "                    && reads_rc_count(tcx, def_id, args.types().next())\n                {\n",
    replace: "                    && reads_rc_count(tcx, def_id, args.types().next())\n                    && false\n                {\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_rc_counted_by_its_consumers_instance"],
  },
  {
    name: "counted-library-class-unseen",
    breaks: "a consumer's copy of its library's generic code, given the library's class whose `Rc` it counts, takes it for no class, so `Rc::new` of it makes a box its readers take for the instance: `chunk.bump is not a function` (#1423)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "        own.chain(counted)\n",
    replace: "        own.chain(counted.filter(|_| false))\n",
    tests: ["test/crates.test.ts", "-t", "^two crates: library_class_rc_made_by_its_generic_code$"],
  },
  {
    name: "mir-identity-unread",
    breaks: "a copy compiled here, MIR alone, compares `Rc<dyn Fn>`s it doesn't count, two `Rc::new`s of one function item one JS function, equal (#1234, ADR 0959)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "                    IdentityRead::Compared(pointee) => _ = asked.insert(erased(pointee)),\n                }\n            }\n            for (from_owner, to_owner)",
    replace: "                    IdentityRead::Compared(_) => {}\n                }\n            }\n            for (from_owner, to_owner)",
    tests: [
      "test/crates.test.ts",
      "-t",
      "^two crates: (rc_fn_identity)$"
    ]
  },
  {
    name: "rc-pointees-of-constants",
    breaks: "a constant's MIR is read as optimized MIR, which rustc refuses, and every library with a generic constant crashes (#1234)",
    file: "src/lower/analysis/rc_counts.rs",
    find: "    if !matches!(tcx.def_kind(id), DefKind::Fn | DefKind::AssocFn) || !tcx.is_mir_available(id) {\n",
    replace: "    if !tcx.is_mir_available(id) {\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^class_methods.rs$"
    ]
  },
];
