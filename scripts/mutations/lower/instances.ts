// Mutations of src/lower/instances.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "closure-placed-by-its-root-alone",
    breaks: "a closure an instance makes of a function rustc inlined into it is looked up by its typeck root alone, `joined`, which no module holds: a panic (#1029)",
    file: "src/lower/instances.rs",
    find: "            .or_else(|| self.placed.get(&id)))\n",
    replace: "            .or(None))\n",
    tests: ["test/crates-compiled-here.test.ts", "-t", "inlined function is placed"],
  },
  {
    name: "closure-of-an-inlined-function-unplaced",
    breaks: "`joined`'s closure, made in the instance `fold_with`, isn't placed where `fold_with` is: a panic of no module for it (#1029)",
    file: "src/lower/instances.rs",
    find: "            made.push((closure, module));\n",
    replace: "",
    tests: ["test/crates-compiled-here.test.ts", "-t", "inlined function is placed"],
  },
  {
    name: "library-changed-unshared",
    breaks:
      "a library's exported function that copies what its consumer changes in place, which the library changes too, so its JS copies it, is compiled by the consumer, where it is imported, `Dep.firstNotch(notches)`; React's Flight client's `close`, refused as it reads its library's private state (rd606)",
    file: "src/lower/instances.rs",
    find: "                .filter(|&&ty| !self.recognition.foreign.changes(library, ty))\n",
    replace: "                .filter(|_| true)\n",
    tests: [
      "test/crates-compiled-here.test.ts",
      "-t",
      "a library's generic function is its program's",
    ],
  },
  {
    name: "instance-constant-not-copied",
    breaks:
      "a library's function that reads its `const SHAPE`, a `Copy` enum its consumer changes in place, isn't taken to copy it, so changing what it gives changes the constant (ADR 0914, amended)",
    file: "src/lower/instances.rs",
    find: "                    mir::Operand::Constant(constant) => self.copies |= (self.holds)(constant.ty()),\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "two crates: copy_enum"],
  },
  {
    name: "instance-box-read-copied",
    breaks:
      "optimized MIR's copy of a `Box` it reads through is taken for a copy of what it holds, and a consumer compiles the library's `grow` itself, `const Tree = { grow(tree) { .. } }`, which it needn't (ADR 0914, amended)",
    file: "src/lower/instances.rs",
    find: "                    mir::Operand::Copy(place) if copy(place.ty(&self.body.local_decls, tcx).ty) => {\n",
    replace:
      "                    mir::Operand::Copy(place) if { let _ = &copy; true } => {\n",
    tests: ["test/crates-compiled-here.test.ts", "-t", "grows a library.s boxed tree"],
  },
  {
    name: "instance-trait-call-unresolved",
    breaks:
      "a library function's call of its trait's method isn't resolved to the impl's that copies a `Spot`, so `first_of` is its library's and shares what it gives (ADR 0914, amended)",
    file: "src/lower/instances.rs",
    find: "                                .map_or(callee, |instance| instance.def_id()),\n",
    replace: "                                .map_or(callee, |_| callee),\n",
    tests: ["test/crates-compiled-here.test.ts", "-t", "program's, alone or a dependency"],
  },
  {
    name: "binding-result-copies",
    breaks:
      "a library's function whose binding call gives a `Vec<String>` the crate pushes to, `js::object::keys(props)`, is taken to copy it, and compiled again in the crate, React's PerformanceTrack's own `addObjectToProperties`",
    file: "src/lower/instances.rs",
    find: "                        Some(callee) if super::bindings::is_binding(tcx, callee) => false,\n",
    replace: "",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "binding's Vec is imported",
    ],
  },
  {
    name: "concat-copies-its-array",
    breaks:
      "a library's function of a slice's `concat()` of `Option<&Unknown>`s, which the crate pushes a `Vec` of, is compiled again, React's addValueToProperties' 620 lines in the reconciler (ADR 0914, amended)",
    file: "src/lower/instances.rs",
    find: "ty::Adt(_, made) => made.types().next().is_some_and(|item| (self.holds)(item)),",
    replace: "ty::Adt(..) => true,",
    tests: ["test/cross-crate-items.test.ts", "-t", "of a concat is imported"],
  },
  {
    name: "concat-items-unasked",
    breaks:
      "a library's function of a `concat()` of `Vec<u32>`s the crate pushes to is imported, its library's JS sharing the arrays it was given, which the crate's push then changes (ADR 0914, amended)",
    file: "src/lower/instances.rs",
    find: "ty::Adt(_, made) => made.types().next().is_some_and(|item| (self.holds)(item)),",
    replace: "ty::Adt(_, made) => made.types().next().is_some_and(|_| false),",
    tests: ["test/cross-crate-items.test.ts", "-t", "of a concat is imported"],
  },
  {
    name: "instance-closure-holds-its-makers-params",
    breaks:
      "a closure a library's generic function gives std holds each of its type parameters, though it captures none, so React DOM copies the reconciler's runWithFiberInDEV, whose `CURRENT.with_borrow(|current| current.clone())` gives std no `()`, with a JS label and drop flags (ADR 0914, amended, gap rd85)",
    file: "src/lower/instances.rs",
    find: "                walk.skip_current_subtree();\n",
    replace: "                continue;\n",
    tests: ["test/crates.test.ts", "-t", "given what may look like None"],
  },
  {
    name: "library-body-copies-asked-monomorphized",
    breaks:
      "what a library's generic function copies is asked with no parameters of its own, so a consumer of one whose `[i32; N]` it iterates crashes rustc, `cannot find N/#0 in param-env` (#454)",
    file: "src/lower/instances.rs",
    find: "            typing_env: ty::TypingEnv::post_analysis(tcx, body.source.def_id()),\n",
    replace: "            typing_env: ty::TypingEnv::fully_monomorphized(),\n",
    tests: ["test/crates.test.ts", "-t", "two crates: const_generic"],
  },
  {
    name: "instance-impl-consts-unasked",
    breaks:
      "a library's trait impl a consumer compiles itself is compiled without its constants, so a library that compiles arrayvec's `impl<T, const CAP: usize> ArrayVecImpl for ArrayVec<T, CAP>`, rust_decimal, is refused at its `CAPACITY = CAP`, which has no initializer to lower (#468, ADR 0176, amended)",
    file: "src/lower/instances.rs",
    find: "                    DefKind::AssocConst { .. } => true,\n",
    replace: "                    DefKind::AssocConst { .. } => false,\n",
    tests: ["test/crates.test.ts", "-t", "three crates: generic_impl_const_of_instance"],
  },
  {
    name: "js-alone-given-counted",
    breaks:
      "`function::name(&signal_oper::<()>)` counts as a call giving `()`, so `signalOper` boxes its `Some` and `signal` binds a copy, `signalOperUnboxed`: `isSignal` is false (#565)",
    file: "src/lower/instances.rs",
    find: "        .filter(|&(callee, _, at)| !alone.iter().any(|&(f, given)| f == callee && given.contains(at)))\n",
    replace: "        .filter(|_| true)\n",
    tests: ["test/rest-parameters.test.ts", "-t", "a generic function of rest arguments JS alone reads is its own JS"],
  },
  {
    name: "js-alone-of-a-function-given-back",
    breaks:
      "a function `function::bind` gives back, which Rust calls, gives its type parameters nothing: `first::<()>`'s `Some(())` isn't boxed, so `is_some()` of it is false (#565)",
    file: "src/lower/instances.rs",
    find: "            && function_itself(tcx, callee, index) == Some(Itself::Unreturned)\n",
    replace: "            && function_itself(tcx, callee, index).is_some()\n",
    tests: ["test/rest-parameters.test.ts", "-t", "a generic function of rest arguments JS alone reads is its own JS"],
  },
  {
    name: "js-alone-of-no-rest",
    breaks:
      "a generic function of no rest arguments given to `function::name` gives its type parameters nothing, though its value is still lowered as its call's copy, which no analysis made: refused (#565)",
    file: "src/lower/instances.rs",
    find: "            && crate::lower::bindings::takes_rest(tcx, f)\n",
    replace: "",
    tests: ["test/rest-parameters.test.ts", "-t", "a generic function of rest arguments JS alone reads is its own JS"],
  },
  {
    name: "closure-constant-functions-uncalled",
    breaks: "a function a closure constant holds, std's `Iterator::last`'s `some` in `FlattenCompat`'s inlined `last` of `Option`s, is given no drop of its type parameter, so `flatten().last()` of `Some`s never drops the one it passes (#645)",
    file: "src/lower/instances.rs",
    find: "            } else if constant.ty().is_closure() {\n",
    replace: "            } else if false {\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "copies-settled-alone",
    breaks:
      "of functions that call each other, only the last asked is settled with the first's answer: the first stays asked, and each asked of it after answers no copy, so `outer`, which calls `inner`, which calls it and one that copies, is the library's, sharing what Rust copies (#884)",
    file: "src/lower/instances.rs",
    find: "            let settled = self.asking.borrow_mut().drain(depth..).collect::<Vec<_>>();\n",
    replace: "            let settled = self.asking.borrow_mut().pop().into_iter().collect::<Vec<_>>();\n",
    tests: [
      "test/crates-compiled-here.test.ts",
      "-t",
      "a library's generic function is its program's",
    ],
  },
  {
    name: "impl-defaults-unasked",
    breaks:
      "whether a library's impl copies what the crate changes asks only the methods it writes, not its trait's defaults its exported dictionary holds, so a generic default's clone of a `Vec` the crate changes is shared (#1023)",
    file: "src/lower/instances.rs",
    find: "                .chain(defaults_used(tcx, id))\n",
    replace: "                .chain(defaults_used(tcx, id).filter(|_| false))\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_own_methods_through_its_dictionary"],
  },
  {
    name: "derive-asked-as-its-library",
    breaks:
      "a library's derive a body calls, which rust-js writes in place of the caller's types, is asked generically as its library's code, so a `Holder<T>`'s clone where `T::Item` is a `Vec` the crate changes is shared (#1023)",
    file: "src/lower/instances.rs",
    find: "                        Some(callee) if self.recognition.foreign.is_library(callee.krate) && !derived(callee) => {\n",
    replace: "                        Some(callee) if self.recognition.foreign.is_library(callee.krate) => {\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_own_methods_through_its_dictionary"],
  },
  {
    name: "rc-param-impl-method-resolved",
    breaks: "a trait impl's generic method that counts an `Rc<T>`, called through its trait, isn't asked what the trait's method's namings give its own `T`, so it's taken for one given nothing, the `Rc` it's given isn't counted and its count is wrong (#1001, #1009)",
    file: "src/lower/instances.rs",
    find: "        if declared != owner {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^(rc_counted_by_a_trait_method|rc_counted_by_callers_instances).rs$"],
  },
  {
    name: "rc-public-callers-seen",
    breaks: "a `pub` generic function's `Rc<T>` counts only what the crate's own calls give it, though JS or a consumer may give it any type, whose `Rc` is then uncounted there (#1009)",
    file: "src/lower/instances.rs",
    find: "        Fact::Shared => !tcx.visibility(owner).is_public(),\n",
    replace: "        Fact::Shared => true,\n",
    tests: ["test/crates.test.ts", "-t", "counts each type"],
  },
  {
    name: "promoted-fn-ref-unseen",
    breaks: "a library's generic function made a `&dyn Fn`, `&dep::by_dyn`, a promoted `&FnDef` in MIR, isn't seen as named, so the consumer's `dyn` is the library's unboxed export and `d(())` is `None` (#1210)",
    file: "src/lower/instances.rs",
    find: "            if let Some(called) = super::fn_def(constant.ty().peel_refs()) {\n",
    replace: "            if let Some(called) = super::fn_def(constant.ty()) {\n",
    tests: ["test/crates.test.ts", "-t", "library_some_through_a_dyn"],
  },
  {
    name: "relayed-copy-lost",
    breaks:
      "a type parameter passed on to a function that copies isn't given a copy itself, so `relay(p)` shares `p`'s object",
    file: "src/lower/instances.rs",
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
    file: "src/lower/instances.rs",
    find: "                    _ if itself(given_ty) => {}\n",
    replace: "",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "judged by what its callers give",
    ],
  },
  {
    name: "trait-method-copies-by-callers",
    breaks:
      "a trait method's `Copy` bound is decided by the crate's direct calls, though it's called through a dictionary",
    file: "src/lower/instances.rs",
    find: "                && tcx.trait_of_assoc(id).is_none()\n                && tcx.trait_impl_of_assoc(id).is_none()\n",
    replace: "                && tcx.trait_impl_of_assoc(id).is_none()\n",
    tests: [
      "test/lowering-functions.test.ts",
      "-t",
      "a Copy bound takes a copy only",
    ],
  },
  {
    name: "trait-impl-copies-by-callers",
    breaks:
      "a trait impl's own `T: Copy` takes no copy function, which its method's dictionary is given, so a copy of an `Option<Point>` its method makes is the value it copies, and a change to it shows there",
    file: "src/lower/instances.rs",
    find: "                && !matches!(tcx.def_kind(id), DefKind::Impl { of_trait: true })\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^trait_impl_copy_bound.rs$"],
  },
  {
    name: "instance-copies-unasked",
    breaks:
      "a library's function the crate compiles itself takes a copy of each `Copy` type parameter, as one a dictionary calls, `pairOf(value, dropped, TCopy, dropD)` where its callers give its own copy (ADR 0914)",
    file: "src/lower/instances.rs",
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
    file: "src/lower/instances.rs",
    find: "                    made.insert((caller, arg));\n                    given.extend(instances_in(tcx, (callers, fact), (arg, caller), (seen, made))?);\n",
    replace: "                    return None;\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_made_in_generic_code_asked_its_hint\\.rs$"],
  },
  {
    name: "program-pub-rc-callers-unseen",
    breaks: "a program's `pub` trait's generic method counts every `Rc` beside it, as a library's does, though JS's values drop nothing and read no count: a neighbour's class constructor's `Rc<Self>` is then an `Rc::new` of a type parameter it may be, refused (#1112)",
    file: "src/lower/instances.rs",
    find: "        Fact::Counted { library } => !library || !tcx.visibility(owner).is_public(),\n",
    replace: "        Fact::Counted { library } => !tcx.visibility(owner).is_public(),\n",
    tests: ["test/neighbours.test.ts", "-t", "#1112"],
  },
  {
    name: "library-pub-rc-callers-seen",
    breaks: "a library's `pub` generic takes its `Rc<T>` to be of what its own crate's calls give alone, though a consumer may give it a value that drops: left uncounted, the consumer's value is never dropped by the last clone (#1009, #1112)",
    file: "src/lower/instances.rs",
    find: "        Fact::Counted { library } => !library || !tcx.visibility(owner).is_public(),\n",
    replace: "        Fact::Counted { library } => library || !library || !tcx.visibility(owner).is_public(),\n",
    tests: ["test/crates.test.ts", "-t", "^two crates: library_rc_(of_what_its_consumer_drops|counted_through_a_traits_generic_method)$"],
  },
];
