//! A class's member's function made its class's (ADR 0761): a method's
//! `self` its `this`, and a constructor's struct literal, or the variable
//! that holds one, its `this`'s fields set. Split from classes.rs by
//! concept, as a file is at most 1500 lines.

use super::{Member, Receiver, body_receiver, class_of, member, object_of, spread_state, superclass};
use crate::collections::IndexSet;
use crate::js::{self, Expr, ExprKind, Pattern, Prop, Stmt, StmtKind};
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;

/// `function` (ADR 0761) of a class's member, lowered as its impl's
/// function: a constructor's `return { props, .. }` its `this`'s fields
/// set, `this.props = props;`. A method's `self` is `this` already, made
/// so before tidy (`receiver_as_this`). `Err` of a constructor that
/// returns what's no struct literal.
pub(in crate::lower) fn as_member(
    tcx: TyCtxt<'_>,
    def_id: DefId,
    class: &str,
    function: &mut js::Function,
) -> Result<(), &'static str> {
    let Some(member) = member(tcx, def_id) else {
        return Ok(());
    };
    match member {
        // Its instance a handle may be kept on the place of, a field's, as
        // a method's `this` (#359).
        Member::Constructor => {
            spread_state(tcx, def_id, function)?;
            set_this(&mut function.body)?;
            this_outside_accessors(class, &mut function.body);
            // Of a class that extends one, its superclass constructs `this` first,
            // `super(props);`, at its name: a comment its body starts with follows.
            if class_of(tcx, def_id).is_some_and(|adt| superclass(tcx, adt).is_some()) {
                let given = (function.params.iter())
                    .map(|param| match param {
                        Pattern::Name(name) => Ok(Expr::var(name)),
                        _ => Err("constructor of a class that extends one names its parameter, `props`, which `super(props)` is given"),
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let first = StmtKind::Expr(Expr::call(Expr::super_(), given)).at(function.name_span);
                function.body.insert(0, first);
            }
            Ok(())
        }
        Member::Method | Member::Getter => {
            this_outside_accessors(class, &mut function.body);
            Ok(())
        }
        // Named as its struct is, its function of no type's; exported only
        // where another module names it, which its callers add (ADR 0943).
        Member::Own => {
            let object =
                object_of(tcx, def_id).map_or_else(|| "object".to_owned(), |adt| tcx.item_name(adt).to_string());
            this_outside_accessors(&object, &mut function.body);
            function.export = false;
            Ok(())
        }
        Member::Static | Member::Const | Member::Namespace => Ok(()),
    }
}

/// Where `class`'s member `name` is written, as `member` is of it: its
/// constructor function, its prototype's method or getter; the rest, its
/// statics and constants, written where they are.
/// Where `name`, a class's `member`, is in `class`: a prototype's method
/// that's the function it passes its parameters on to, `forwarding`, that
/// function where its JS is that call alone (`js::Class::forwarded`, rd715).
pub(in crate::lower) fn place((member, forwarding): (Member, bool), class: &mut js::Class, name: &str) {
    if forwarding && member == Member::Method {
        class.forwarding.push(name.to_owned());
    }
    match member {
        Member::Constructor => class.constructor = name.to_owned(),
        Member::Method => class.prototype.push(name.to_owned()),
        Member::Getter => class.getters.push(name.to_owned()),
        Member::Static | Member::Const | Member::Own | Member::Namespace => {}
    }
}

/// Of `body`, a class's method's: each handle kept on a place of `this`
/// on the instance named before it, `const counted = this;`,
/// `counted.reads` (#359, `js::this_outside_accessors`). Named as its class
/// is, as a method's `self` is.
fn this_outside_accessors(class: &str, body: &mut Vec<Stmt>) {
    let mut instance = class.to_owned();
    instance[..1].make_ascii_lowercase();
    js::this_outside_accessors(body, &instance);
}

/// Of `function`, a class's method, its `self` parameter, `this` in its
/// body, `params` left without it. Before tidy, which would take a `let`
/// of its name, read no more after, for it (ADR 0650): `let complex_number
/// = ..` of a `ComplexNumber`'s method, whose `self` is named
/// `complexNumber` too, would be `this = ..`. And the paths of `booleans`
/// through it, `comparator.lessThan()`'s `this.lessThan()`, which tidy
/// reads `this`'s by.
pub(in crate::lower) fn receiver_as_this(
    tcx: TyCtxt<'_>,
    function: DefId,
    params: &mut Vec<Pattern>,
    body: &mut Vec<Stmt>,
    booleans: &mut IndexSet<String>,
) -> Result<(), &'static str> {
    // A namespace's method has no `self`: what it calls through it is called
    // by its name (rd292, ADR 1037), and nothing else reads it.
    let taken = body_receiver(tcx, function);
    if taken == Receiver::Absent {
        let Some(Pattern::Name(receiver)) = params.first() else {
            return Err("a method whose `self` is no parameter");
        };
        let mut read = false;
        js::visit_stmts(body, &mut |name| read |= name == receiver);
        if read {
            return Err(
                "`#[rust_js::namespace]` method that reads `self` only to call its functions, `self.update_container(..)`: its module has no value of its own (ADR 1037)",
            );
        }
        params.remove(0);
        return Ok(());
    }
    if taken != Receiver::This {
        return Ok(());
    }
    let receiver = match params.first() {
        Some(Pattern::Name(name)) => name.clone(),
        _ => return Err("a method whose `self` is no parameter"),
    };
    params.remove(0);
    // A function nested in it, `js::function::with_this`'s, has a `this`
    // of its own: it reads `self` as the method's, a `const` of it.
    if js::read_as_this(body, &receiver) {
        body.insert(0, StmtKind::Const(receiver.clone(), Expr::this()).at(js::Span::NONE));
    }
    let through = format!("{receiver}.");
    *booleans = std::mem::take(booleans)
        .into_iter()
        .map(|path| match path.strip_prefix(&through) {
            Some(rest) => format!("this.{rest}"),
            None => path,
        })
        .collect();
    Ok(())
}

/// The assignments to `this` of a constructor's struct literal's fields,
/// `this.a = a;`, each where its value is, or at `span`: not of a field
/// that's `undefined`, a `None`, which an object leaves out (ADR 0280), as
/// JS's constructor function sets no property it hasn't a value of.
fn fields_set(props: Vec<Prop>, layout: &js::ObjectLayout, span: js::Span) -> Result<Vec<Stmt>, &'static str> {
    let mut props = props;
    js::undefined_keys::left_out(&mut props, layout);
    let mut out = Vec::with_capacity(props.len());
    for prop in props {
        match prop {
            Prop::Field(name, value) => {
                // Where its value is, which its comment is before.
                let at = if value.span.is_none() { span } else { value.span };
                out.push(StmtKind::Assign(Expr::member(Expr::this(), name), value).at(at));
            }
            _ => return Err("a constructor whose struct literal has a field of no name"),
        }
    }
    Ok(out)
}

/// Of a constructor's `body`, the variable its last statement returns,
/// made of a struct literal where it's declared and never set again,
/// `let fiber = Fiber { .. }; if on { fiber.x.set(..); } fiber`: that
/// value is `this`, its literal's fields set on it where it's declared,
/// each read of it `this`, `Object.preventExtensions(this)`, and each
/// `return fiber` `return;`, the body's last none (ADR 0761, amended).
fn instance_as_this(body: &mut Vec<Stmt>) -> Result<(), &'static str> {
    let Some(Stmt {
        kind: StmtKind::Return(Some(Expr {
            kind: ExprKind::Var(instance),
            ..
        })),
        ..
    }) = body.last()
    else {
        return Ok(());
    };
    let instance = instance.clone();
    let Some(at) = body.iter().position(|stmt| {
        matches!(&stmt.kind,
            StmtKind::Const(name, value) | StmtKind::Let(name, Some(value))
                if *name == instance && matches!(value.kind, ExprKind::Object(..)))
    }) else {
        return Ok(());
    };
    if js::Writes::sets(body, &instance) {
        return Err("a constructor whose instance is set again, not changed in place");
    }
    let made = body.remove(at);
    let (StmtKind::Const(_, value) | StmtKind::Let(_, Some(value))) = made.kind else {
        unreachable!("the instance's declaration")
    };
    let ExprKind::Object(props, layout) = value.kind else {
        unreachable!("a struct literal")
    };
    let fields = fields_set(props, &layout, made.span)?;
    body.splice(at..at, fields);
    body.pop();
    js::statement_lists(body, &mut |list| {
        for stmt in list.iter_mut() {
            if matches!(&stmt.kind, StmtKind::Return(Some(Expr { kind: ExprKind::Var(v), .. })) if *v == instance) {
                stmt.kind = StmtKind::Return(None);
            }
        }
    });
    if js::read_as_this(body, &instance) {
        return Err("a constructor whose instance a function made in it reads, whose `this` is its own");
    }
    Ok(())
}

/// Each `return { a, b }` of `body` its fields set on `this`, `this.a =
/// a; this.b = b;`, and `return;` where it's not the body's last; not a
/// closure's, whose `return` is its own. Or the instance a variable holds,
/// `instance_as_this`.
fn set_this(body: &mut Vec<Stmt>) -> Result<(), &'static str> {
    instance_as_this(body)?;
    let mut fine = Ok(());
    let mut outermost = true;
    js::statement_lists(body, &mut |list| {
        let last = std::mem::take(&mut outermost).then_some(list.len());
        let mut out = Vec::with_capacity(list.len());
        for (i, stmt) in std::mem::take(list).into_iter().enumerate() {
            if !matches!(&stmt.kind, StmtKind::Return(Some(value)) if matches!(value.kind, ExprKind::Object(..))) {
                if matches!(stmt.kind, StmtKind::Return(Some(_))) {
                    fine = Err("a constructor that gives what's no struct literal, `Self { .. }`");
                }
                out.push(stmt);
                continue;
            }
            let span = stmt.span;
            let StmtKind::Return(Some(Expr {
                kind: ExprKind::Object(props, layout),
                ..
            })) = stmt.kind
            else {
                unreachable!("a struct literal returned")
            };
            match fields_set(props, &layout, span) {
                Ok(fields) => out.extend(fields),
                Err(why) => fine = Err(why),
            }
            if Some(i + 1) != last {
                out.push(StmtKind::Return(None).at(span));
            }
        }
        *list = out;
    });
    fine
}
