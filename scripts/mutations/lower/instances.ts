// Mutations of crates/rust-js-lower/src/instances.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "closure-placed-by-its-root-alone",
    breaks: "a closure an instance makes of a function rustc inlined into it is looked up by its typeck root alone, `joined`, which no module holds: a panic (#1029)",
    file: "crates/rust-js-lower/src/instances.rs",
    find: "            .or_else(|| self.placed.get(&id)))\n",
    replace: "            .or(None))\n",
    tests: ["test/crates-compiled-here.test.ts", "-t", "inlined function is placed"],
  },
  {
    name: "closure-of-an-inlined-function-unplaced",
    breaks: "`joined`'s closure, made in the instance `fold_with`, isn't placed where `fold_with` is: a panic of no module for it (#1029)",
    file: "crates/rust-js-lower/src/instances.rs",
    find: "            made.push((closure, module));\n",
    replace: "",
    tests: ["test/crates-compiled-here.test.ts", "-t", "inlined function is placed"],
  },
  {
    name: "instance-closure-holds-its-makers-params",
    breaks:
      "a closure a library's generic function gives std holds each of its type parameters, though it captures none, so React DOM copies the reconciler's runWithFiberInDEV, whose `CURRENT.with_borrow(|current| current.clone())` gives std no `()`, with a JS label and drop flags (ADR 0914, amended, gap rd85)",
    file: "crates/rust-js-lower/src/instances.rs",
    find: "                walk.skip_current_subtree();\n",
    replace: "                continue;\n",
    tests: ["test/crates.test.ts", "-t", "given what may look like None"],
  },
  {
    name: "instance-impl-consts-unasked",
    breaks:
      "a library's trait impl a consumer compiles itself is compiled without its constants, so a library that compiles arrayvec's `impl<T, const CAP: usize> ArrayVecImpl for ArrayVec<T, CAP>`, rust_decimal, is refused at its `CAPACITY = CAP`, which has no initializer to lower (#468, ADR 0176, amended)",
    file: "crates/rust-js-lower/src/instances.rs",
    find: "                    DefKind::AssocConst => true,\n",
    replace: "                    DefKind::AssocConst => false,\n",
    tests: ["test/crates.test.ts", "-t", "three crates: generic_impl_const_of_instance"],
  },
  {
    name: "js-alone-given-counted",
    breaks:
      "`function::name(&signal_oper::<()>)` counts as a call giving `()`, so `signalOper` boxes its `Some` and `signal` binds a copy, `signalOperUnboxed`: `isSignal` is false (#565)",
    file: "crates/rust-js-lower/src/instances.rs",
    find: "        .filter(|&(callee, _, at, _)| !alone.iter().any(|&(f, given)| f == callee && given.contains(at)))\n",
    replace: "        .filter(|_| true)\n",
    tests: ["test/rest-parameters.test.ts", "-t", "a generic function of rest arguments JS alone reads is its own JS"],
  },
  {
    name: "js-alone-of-a-function-given-back",
    breaks:
      "a function `function::bind` gives back, which Rust calls, gives its type parameters nothing: `first::<()>`'s `Some(())` isn't boxed, so `is_some()` of it is false (#565)",
    file: "crates/rust-js-lower/src/instances.rs",
    find: "            && function_itself(tcx, callee, index) == Some(Itself::Unreturned)\n",
    replace: "            && function_itself(tcx, callee, index).is_some()\n",
    tests: ["test/rest-parameters.test.ts", "-t", "a generic function of rest arguments JS alone reads is its own JS"],
  },
  {
    name: "js-alone-of-no-rest",
    breaks:
      "a generic function of no rest arguments given to `function::name` gives its type parameters nothing, though its value is still lowered as its call's copy, which no analysis made: refused (#565)",
    file: "crates/rust-js-lower/src/instances.rs",
    find: "            && crate::bindings::takes_rest(tcx, f)\n",
    replace: "",
    tests: ["test/rest-parameters.test.ts", "-t", "a generic function of rest arguments JS alone reads is its own JS"],
  },
  {
    name: "rc-param-impl-method-resolved",
    breaks: "a trait impl's generic method that counts an `Rc<T>`, called through its trait, isn't asked what the trait's method's namings give its own `T`, so it's taken for one given nothing, the `Rc` it's given isn't counted and its count is wrong (#1001, #1009)",
    file: "crates/rust-js-lower/src/instances.rs",
    find: "            Some(own) if declared != owner => {\n",
    replace: "            Some(own) if false && declared != owner => {\n",
    tests: ["test/corpus.test.ts", "-t", "^(rc_counted_by_a_trait_method|rc_counted_by_callers_instances).rs$"],
  },
  {
    name: "rc-public-callers-seen",
    breaks: "a `pub` generic function's adapter of what it's given is taken to be made of what its own crate's calls give alone, though a consumer may give it another, whose hint it then doesn't ask as std does (`Fact::Shared`, #1236)",
    file: "crates/rust-js-lower/src/instances.rs",
    find: "        Fact::Shared => !crate::declarations::js_names(tcx, owner),\n",
    replace: "        Fact::Shared => true,\n",
    tests: ["test/corpus.test.ts", "-t", "^compared_keeps_its_dictionary.rs$"],
    snapshots: true,
  },
  {
    name: "promoted-fn-ref-unseen",
    breaks: "a library's generic function made a `&dyn Fn`, `&dep::by_dyn`, a promoted `&FnDef` in MIR, isn't seen as named, so the consumer's `dyn` is the library's unboxed export and `d(())` is `None` (#1210)",
    file: "crates/rust-js-lower/src/instances.rs",
    find: "            if let Some(called) = super::fn_def(constant.ty().peel_refs()) {\n",
    replace: "            if let Some(called) = super::fn_def(constant.ty()) {\n",
    tests: ["test/crates.test.ts", "-t", "library_some_through_a_dyn"],
  },
  {
    name: "relayed-copy-lost",
    breaks:
      "a type parameter passed on to a function that copies isn't given a copy itself, so `relay(p)` shares `p`'s object",
    file: "crates/rust-js-lower/src/instances.rs",
    find: "            if given.contains(&from) && given.insert(to) {",
    replace: "            if false && given.insert(to) {",
    tests: [
      "test/lowering-functions.test.ts",
      "-t",
      "a Copy bound takes a copy only",
    ],
  },
  {
    name: "clone-given-of-its-own-clone",
    breaks:
      "a `T: Clone` given only what's its own clone, `Option<Rc<Cache>>`, still takes a `TClone`, and its clone under a library's unchecked borrow is refused (ADR 0914, amended)",
    file: "crates/rust-js-lower/src/instances.rs",
    find: "                    _ if itself(given_ty) => {}\n",
    replace: "",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "judged by what its callers give",
    ],
  },
  {
    name: "trait-impl-copies-by-callers",
    breaks:
      "a trait impl's own `T: Copy` takes no copy function, which its method's dictionary is given, so a copy of an `Option<Point>` its method makes is the value it copies, and a change to it shows there",
    file: "crates/rust-js-lower/src/instances.rs",
    find: "                && !matches!(tcx.def_kind(id), DefKind::Impl { of_trait: true })\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^trait_impl_copy_bound.rs$"],
  },
  {
    name: "instance-copies-unasked",
    breaks:
      "a library's function the crate compiles itself takes a copy of each `Copy` type parameter, as one a dictionary calls, `pairOf(value, dropped, TCopy, dropD)` where its callers give its own copy (ADR 0914)",
    file: "crates/rust-js-lower/src/instances.rs",
    find: "            (id.is_local() || foreign.is_instance(id))\n",
    replace: "            id.is_local()\n",
    tests: [
      "test/crates-compiled-here.test.ts",
      "-t",
      "is its program's, alone or a dependency",
    ],
  },
  {
    name: "made-of-own-parameters-unresolved",
    breaks: "what a generic caller makes of its own type parameters and gives on, `Map<Take<I>, F>`, is given no instances, so nothing it's made of is made knowing its hint (#1240)",
    file: "crates/rust-js-lower/src/instances.rs",
    find: "                made.insert((caller, arg));\n                given.extend(instances_in(tcx, (callers, fact), (arg, caller), (seen, made))?);\n",
    replace: "                return None;\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_made_in_generic_code_asked_its_hint\\.rs$"],
  },
  {
    name: "program-pub-rc-callers-unseen",
    breaks: "a program's `pub` generic counts every `Rc` beside it, as a library's does, though JS's values drop nothing and read no count: a neighbour's untagged enum of a `RefCell` then holds an `Rc` counted, which it can't tell apart, and the two run otherwise than native Rust together (#1112, #1060's neighbours)",
    file: "crates/rust-js-lower/src/instances.rs",
    find: "            !library\n                || !crate::declarations::js_names(tcx, owner)\n                || consumer_compiles(tcx, tcx.typeck_root_def_id(owner))\n",
    replace: "            !crate::declarations::js_names(tcx, owner)\n",
    tests: ["test/neighbours.test.ts", "-t", "#1060"],
  },
  {
    name: "library-pub-rc-callers-seen",
    breaks: "a library's `pub` trait method, which a dictionary calls and no consumer compiles its own of, takes its `Rc<T>` to be of what its own crate's calls give alone (#1233's ground, #1234), though a consumer may give it a value that drops: left uncounted, the consumer's value is never dropped by the last clone (#1009, #1112)",
    file: "crates/rust-js-lower/src/instances.rs",
    find: "            !library\n                || !crate::declarations::js_names(tcx, owner)\n                || consumer_compiles(tcx, tcx.typeck_root_def_id(owner))\n",
    replace: "            !library || !crate::declarations::js_names(tcx, owner) || true\n",
    tests: ["test/crates.test.ts", "-t", "^two crates: library_rc_(of_what_its_consumer_drops|counted_through_a_traits_generic_method)$"],
  },
  {
    name: "library-pub-compilable-rc-callers-unseen",
    breaks: "a library's `pub` function a consumer can compile takes its callers as unseen, so its `Rc<T>` counts every `Rc` as before, and its contract a shape of every `Slot` (#1234)",
    file: "crates/rust-js-lower/src/instances.rs",
    find: "            !library\n                || !crate::declarations::js_names(tcx, owner)\n                || consumer_compiles(tcx, tcx.typeck_root_def_id(owner))\n",
    replace: "            !library || !crate::declarations::js_names(tcx, owner)\n",
    tests: [
      "test/crates.test.ts",
      "-t",
      "a library counts each type's Rcs apart"
    ]
  },
  {
    name: "trait-family-unfollowed",
    breaks: "a method of the crate's own trait takes its callers as unseen, so it takes a copy function, a `Clone` dictionary and boxes its `Some`s whatever its callers give (#1233)",
    file: "crates/rust-js-lower/src/instances.rs",
    find: "                    || method_family(tcx, id).is_some())\n",
    replace: "                    || false)\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^trait_method_given_by_its_callers.rs$"
    ],
    snapshots: true
  },
  {
    name: "trait-family-declaration-unmarked",
    breaks: "a trait's declaration of a method, which no item is, isn't marked with its impls, so a call through it passes no copy function an impl's method takes, refused for want of one (#1233)",
    file: "crates/rust-js-lower/src/instances.rs",
    find: "                    .filter(|&(member, at)| bounded(member).contains(&at))\n",
    replace: "                    .filter(|&(member, at)| is_item(member) && bounded(member).contains(&at))\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^trait_method_given_by_its_callers.rs$"
    ]
  },
  {
    name: "trait-impl-params-seen",
    breaks: "an impl's own parameter, which a call through its trait names only as `Self`'s, is taken as seen, so `Some(self.0.clone())` of a `Wrap<Option<u8>>` holding `None` is `None` (#1233)",
    file: "crates/rust-js-lower/src/instances.rs",
    find: "        } else if method_family(tcx, id).is_some() {\n",
    replace: "        } else if false {\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^trait_method_given_by_its_callers.rs$"
    ]
  },
  {
    name: "trait-family-namings-unshared",
    breaks: "a naming of a trait's method decides for itself alone, not its family's, so a call through the trait passes a copy function an impl's method doesn't take, and copies nothing (#1233)",
    file: "crates/rust-js-lower/src/instances.rs",
    find: "                Some(own) => (family.iter())\n",
    replace: "                Some(own) => (family.iter().take(1))\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^trait_method_given_by_its_callers.rs$"
    ]
  },
  {
    name: "called-own-trait-impl-unmatched",
    breaks: "an impl's own parameter, of the crate's own trait, `F` of `impl<F: FnOnce() -> u32> Run for Once<F>`, isn't matched in what the trait's method's namings give, so a closure moved into `Once` and called there by `FnOnce` is never dropped (#2031)",
    file: "crates/rust-js-lower/src/instances.rs",
    find: "                            return None;\n                        }\n                        givings.push(Giving::Matched { caller, given, pattern });\n",
    replace: "                            return None;\n                        }\n",
    tests: ["test/corpus.test.ts", "-t", "^fn_closure_moved_into_a_struct_called_once_refused.rs$"],
  },
  {
    name: "called-held-impl-unmatched",
    breaks: "an impl's own parameter, of another crate's trait std calls unseen, `Drop` of a `Guard<F>`, isn't matched in what the crate's bodies hold, so a closure a guard's drop calls by `FnOnce` is never dropped (#2031)",
    file: "crates/rust-js-lower/src/instances.rs",
    find: "                    for &(caller, given) in held?.get(&adt.did()).into_iter().flatten() {\n",
    replace: "                    for &(caller, given) in held?.get(&adt.did()).into_iter().flatten().filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^fn_closure_run_by_a_guards_drop_refused.rs$"],
  },
  {
    name: "called-matched-param-undeferred",
    breaks: "an impl's type matched with a caller's type parameter, `R` of `run_it<R: Run>`, isn't matched in that parameter's values in turn, so a closure moved into the `Once` given for it and called by `FnOnce` is never dropped (#2031)",
    file: "crates/rust-js-lower/src/instances.rs",
    find: "                        todo.push((caller, param, by_ref, [rest, &[(part, index)]].concat()))\n",
    replace: "                        _ = (caller, param, by_ref, rest, part);\n",
    tests: ["test/corpus.test.ts", "-t", "^fn_closure_moved_into_a_struct_called_once_refused.rs$"],
  },
  {
    name: "called-box-held-taken-as-given",
    breaks: "a boxed `Fn` matched in a struct a call by value calls it in, `Once<F>` of `impl<F: FnOnce() -> u32> Run for Once<F>`, is taken as the call's own argument, so it isn't refused and its box is never dropped (#2050)",
    file: "crates/rust-js-lower/src/instances.rs",
    find: "            if !flow(owner, ty, (by_ref, As::Held), &pending, &mut todo, on) {\n",
    replace: "            if !flow(owner, ty, (by_ref, As::Called), &pending, &mut todo, on) {\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_fn_moved_into_a_struct_called_once_refused.rs$"],
  },
  {
    name: "std-consumer-impl-unnamed",
    breaks:
      "a `collect()` or `sum()` into a collection of the crate's names its own `from_iter` or `sum` with nothing, so the callers' carrier never sees what it's given and a `filter`'s hint is JS's, `(0, None)`, not std's (#2376)",
    file: "crates/rust-js-lower/src/instances.rs",
    find: "            let implied = crate::recognition::crate_impl_called_by(tcx, typing_env, callee, args);\n",
    replace: "            let implied: Option<(DefId, ty::GenericArgsRef<'tcx>)> = None;\n",
    tests: ["test/corpus.test.ts", "-t", "^hint_asked_by_a_crates_own_collect_and_sum.rs$"],
  },
];
