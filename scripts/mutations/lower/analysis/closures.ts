// Mutations of src/lower/analysis/closures.rs (ADRs 0521, 0522).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "param-owned-through-a-fn-dyn",
    breaks:
      "a struct holding `Box<dyn Fn(&K)>` is taken to own a `K`, so a generic `Rc` of it is refused once the crate has a `Drop` (#497)",
    file: "src/lower/analysis/closures.rs",
    find: "        ty::Ref(..) | ty::FnPtr(..) | ty::FnDef(..) | ty::Dynamic(..) => false,\n",
    replace: "        ty::Ref(..) | ty::FnPtr(..) | ty::FnDef(..) => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_of_functions_of_a_param.rs$"],
  },
  {
    name: "param-owned-through-a-raw-pointer",
    breaks: "an `Rc`'s `NonNull` is taken to own what it points at, so a generic `Rc` of functions is refused (#497)",
    file: "src/lower/analysis/closures.rs",
    find: "    if !ty.has_non_region_param() || !seen.insert(ty) || raw_pointee(tcx, ty).is_some() {\n",
    replace: "    if !ty.has_non_region_param() || !seen.insert(ty) {\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_of_functions_of_a_param.rs$"],
  },
  {
    name: "param-unowned-through-phantom-data",
    breaks:
      "a generic closure holding a `Vec<K>` made a `dyn Fn` by generic code is taken to hold no `K`, so the `dyn Fn` drops nothing of one with a destructor",
    file: "src/lower/analysis/closures.rs",
    find: "        ty::Adt(adt, args) if adt.is_box() || adt.is_phantom_data() => owns_a_param(tcx, args.type_at(0), seen),\n",
    replace: "        ty::Adt(adt, args) if adt.is_box() => owns_a_param(tcx, args.type_at(0), seen),\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_generic_owned_drops.rs$"],
  },
  {
    name: "fn-dyn-drops-unseen",
    breaks:
      "a function holding a value with a destructor made a `dyn Fn` leaves the `dyn Fn` dropping nothing",
    file: "src/lower/analysis/closures.rs",
    find: "            if drops(function) {\n",
    replace: "            if false && drops(function) {\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_drops.rs$"],
  },
  {
    name: "fn-dyn-generic-unseen",
    breaks:
      "a type parameter made a `dyn Fn`, given a closure holding a value with a destructor, leaves the `dyn Fn` dropping nothing",
    file: "src/lower/analysis/closures.rs",
    find: "            (holds_a_param(tcx, function) && closure_drops()).then_some(DroppingFnDyn {\n",
    replace: "            (false && holds_a_param(tcx, function) && closure_drops()).then_some(DroppingFnDyn {\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_generic_drops.rs$"],
  },
  {
    name: "dropping-fn-dyn-holds-itself",
    breaks:
      "a refused `dyn Fn` of a closure names the closure's type as what it holds, not the capture that drops, and so not the owner handle's cell whose call's return doesn't own it (rd553, ADR 0626, amended)",
    file: "src/lower/analysis/closures.rs",
    find: "                    held: held.unwrap_or(function),\n",
    replace: "                    held: function,\n",
    tests: ["test/crates.test.ts", "-t", "callback is a public field"],
  },
  {
    name: "any-dyn-drops-unseen",
    breaks:
      "a value with a destructor made a `dyn Any`, a payload `resume_unwind` throws, leaves the `dyn Any` dropping nothing",
    file: "src/lower/analysis/closures.rs",
    find: "            .any(|from| drops(from) || params_may_drop && from.has_non_region_param())\n",
    replace:
      "            .any(|from| params_may_drop && from.has_non_region_param())\n",
    tests: ["test/corpus.test.ts", "-t", "^catch_unwind_drops.rs$"],
  },
  {
    name: "any-dyn-generic-unseen",
    breaks:
      "a type parameter's value made a `dyn Any`, given one with a destructor, leaves the `dyn Any` dropping nothing",
    file: "src/lower/analysis/closures.rs",
    find: "            .any(|from| drops(from) || params_may_drop && from.has_non_region_param())\n",
    replace: "            .any(|from| drops(from))\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_any_drops_by_need.rs$"],
  },
  {
    name: "fn-dyn-closure-drops-unseen",
    breaks:
      "a closure holding a value with a destructor, given to a type parameter made a `dyn Fn`, isn't seen to drop",
    file: "src/lower/analysis/closures.rs",
    find: "                    drops(ty) || params_may_drop && holds_a_param(tcx, ty)\n",
    replace: "                    params_may_drop && holds_a_param(tcx, ty)\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_generic_drops.rs$"],
  },
  {
    name: "fn-dyn-closure-uncarried",
    breaks:
      "a closure made a `dyn Fn` doesn't carry its drop, and `Box::new` of it is refused",
    file: "src/lower/analysis/closures.rs",
    find: "        self.made_fn_dyns.contains(&closure) || self.cloned_apart.contains(&closure) || given_back(tcx, closure)\n",
    replace:
      "        self.cloned_apart.contains(&closure) || given_back(tcx, closure)\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_drops.rs$"],
  },
  {
    name: "clone-shares-dropping",
    breaks:
      "a clone of a closure holding a counted `Rc` is the closure itself, counting nothing",
    file: "src/lower/analysis/closures.rs",
    find: "    !writes && (closure.upvar_tys().iter()).all(|t| t.is_freeze(tcx, typing_env) && !drops(t))\n",
    replace:
      "    !writes && (closure.upvar_tys().iter()).all(|t| t.is_freeze(tcx, typing_env))\n",
    tests: ["test/corpus.test.ts", "-t", "^closure_clone_counted.rs$"],
  },
  {
    name: "clone-shares-cell",
    breaks: "a clone of a closure holding a `Cell` shares it",
    file: "src/lower/analysis/closures.rs",
    find: "    !writes && (closure.upvar_tys().iter()).all(|t| t.is_freeze(tcx, typing_env) && !drops(t))\n",
    replace: "    !writes && (closure.upvar_tys().iter()).all(|t| !drops(t))\n",
    tests: ["test/corpus.test.ts", "-t", "^closure_clone_cell.rs$"],
  },
  {
    name: "cloned-apart-uncarried",
    breaks:
      "a closure cloned apart doesn't carry its drop, and each clone drops what its maker took",
    file: "src/lower/analysis/closures.rs",
    find: "        self.made_fn_dyns.contains(&closure) || self.cloned_apart.contains(&closure) || given_back(tcx, closure)\n",
    replace:
      "        self.made_fn_dyns.contains(&closure) || given_back(tcx, closure)\n",
    tests: ["test/corpus.test.ts", "-t", "^closure_clone_counted.rs$"],
  },
  {
    name: "functions-clone-shared",
    breaks:
      "a type parameter's function is cloned as itself where a closure is cloned apart",
    file: "src/lower/analysis/closures.rs",
    find: "        self.library || !self.cloned_apart.is_empty()\n",
    replace: "        self.library\n",
    tests: ["test/corpus.test.ts", "-t", "^closure_clone_counted.rs$"],
  },
  {
    name: "clone-shares-by-kind",
    breaks:
      "a closure that writes nothing it took, given to a `map` and so an `FnMut`, is made by a factory to be cloned",
    file: "src/lower/analysis/closures.rs",
    find: "            ty::UpvarCapture::ByValue | ty::UpvarCapture::ByUse => captured.mutability == ty::Mutability::Mut,\n",
    replace:
      "            ty::UpvarCapture::ByValue | ty::UpvarCapture::ByUse => true,\n",
    tests: ["test/corpus.test.ts", "-t", "^lazy_iterator_clone.rs$"],
    snapshots: true,
  },
  {
    name: "fn-once-clone-shares",
    breaks:
      "a clone of a closure that moves what it took out is the closure itself, so both give, and change, the one value",
    file: "src/lower/analysis/closures.rs",
    find: "    } || closure.kind() == ty::ClosureKind::FnOnce;\n",
    replace: "    };\n",
    tests: ["test/corpus.test.ts", "-t", "^closure_once_cloned.rs$"],
  },
  {
    name: "handed-fn-dyn-still-drops",
    breaks:
      "a closure handed to JS still counts as a `dyn Fn` that drops, so a crate whose only such closures JS keeps disposes every `dyn Fn` it drops (ADR 0873)",
    file: "src/lower/analysis/closures.rs",
    find: "            if handed {\n                return None;\n            }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_handed_only.rs$"],
    snapshots: true,
  },
  {
    name: "handed-to-what-gives-back",
    breaks:
      "a closure given to a binding that gives a function back, `widen`'s `this`, is taken as JS's, and dropped as nothing once it's Rust's again (ADR 0873)",
    file: "src/lower/analysis/closures.rs",
    find: "        if !bindings::is_binding(tcx, id) || gives_back_a_function(tcx, id) {\n",
    replace: "        if !bindings::is_binding(tcx, id) {\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_handed_to_js.rs$"],
  },
  {
    name: "handed-before-what-unwinds",
    breaks:
      "a closure boxed for a binding before an argument that panics is taken as JS's, and not dropped as that unwinds (ADR 0873)",
    file: "src/lower/analysis/closures.rs",
    find: "                && args[i + 1..].iter().all(|&after| cannot_unwind(thir, after))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_handed_to_js.rs$"],
  },
  {
    name: "handed-to-js-unseen",
    breaks:
      "a closure handed to JS carries its drop, `$dropping(f, drop)`, which nothing runs (ADR 0873)",
    file: "src/lower/analysis/closures.rs",
    find: "        self.handed_fn_dyns.contains(&closure)\n",
    replace: "        false\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_handed_to_js.rs$"],
    snapshots: true,
  },
  {
    name: "written-back-unseen",
    breaks:
      "a closure given to a binding that writes a function through a `&mut` it's given, `Reflect.set`'s slot, is taken as JS's, and dropped as nothing once it's Rust's again (ADR 0873)",
    file: "src/lower/analysis/closures.rs",
    find: "    holds(sig.output())\n        || (sig.inputs().iter()).any(|input| {\n",
    replace:
      "    holds(sig.output())\n        || false && (sig.inputs().iter()).any(|input| {\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_handed_to_js.rs$"],
  },
  {
    name: "generic-closure-holds-a-param",
    breaks:
      "a closure a generic function boxes holding only its own `Rc`, chainThenableValue<T>'s, is taken to hold a type parameter's value, and every `dyn Fn` of the crate drops (ADR 0521, amended)",
    file: "src/lower/analysis/closures.rs",
    find: "        ty::Closure(_, args) => (args.as_closure().upvar_tys().iter()).any(|t| owns_a_param(tcx, t, seen)),\n",
    replace: "        ty::Closure(..) => true,\n",
    tests: [
      "test/cross-crate-dyn-fns.test.ts",
      "-t",
      "a library's dyn Fn drops in its consumer only where",
    ],
  },
  {
    name: "called-once-closures-unseen",
    breaks:
      "a closure rustc infers `Fn`, given where it's made to a binding that calls it once, `js::set_timeout(move || { let m = &n; .. }, ms)`'s, isn't found, so it's refused, as one JS keeps and calls again would be (ADR 1020)",
    file: "src/lower/analysis/closures.rs",
    find: "                && Taker::parameter(tcx, id, i) == Some(Taker::OnceCallback)\n",
    replace:
      "                && Taker::parameter(tcx, id, i) == Some(Taker::Handler)\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^called_once_closure_drops_after.rs$",
    ],
  },
  {
    name: "called-once-of-fn-once-closures",
    breaks:
      "a closure rustc infers `FnOnce`, which drops what it took in its own body, is taken for one called once that reads itself by reference, and refused (ADR 1020)",
    file: "src/lower/analysis/closures.rs",
    find: "                && closure_args.as_closure().kind() != ty::ClosureKind::FnOnce\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^called_once_closure_drops_after.rs$",
    ],
  },
  {
    name: "fixed-borrow-of-mut-binding",
    breaks: "a `mut` binding a closure borrowed is fixed in it, so a clone of it made there reads the value the parameter is given after: `20 + 2` where Rust says `21` (rd515, ADR 0846, amended)",
    file: "src/lower/analysis/closures.rs",
    find: "        matches!(pat.kind, PatKind::Binding(BindingMode(ByRef::No, Mutability::Not), ..))\n",
    replace: "        matches!(pat.kind, PatKind::Binding(BindingMode(ByRef::No, _), ..))\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_of_a_borrowed_parameter.rs$"],
  },
];
