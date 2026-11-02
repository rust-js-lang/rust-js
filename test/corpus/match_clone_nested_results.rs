// A `match` of a token taken by `?` and cloned, a `String` variant first,
// each of two arms another `match` of a token taken by `?` that gives
// `Ok` or `Err`, and a `_` arm's `Err`: each arm returns where it's
// written, and nothing follows its `return` (#461, where tidy split an
// arm off by the `?`'s `return` after a branch that returned already).
#[derive(Clone, Debug)]
enum Tok {
    Ident(String),
    LParen,
    RParen,
}

#[derive(Debug)]
enum Expr {
    Num(f64),
    Var(String),
}

fn parse_primary(toks: &[Tok]) -> Result<Expr, ()> {
    let mut i = 0;
    let bump = |i: &mut usize| -> Result<&Tok, ()> {
        let t = toks.get(*i).ok_or(())?;
        *i += 1;
        Ok(t)
    };
    match bump(&mut i)?.clone() {
        Tok::LParen => match bump(&mut i)? {
            Tok::RParen => Ok(Expr::Num(0.0)),
            _ => Err(()),
        },
        Tok::Ident(name) => match bump(&mut i)? {
            Tok::RParen => Ok(Expr::Var(name)),
            _ => Err(()),
        },
        _ => Err(()),
    }
}

fn main() {
    let inputs = [
        vec![],
        vec![Tok::LParen],
        vec![Tok::LParen, Tok::RParen],
        vec![Tok::LParen, Tok::LParen],
        vec![Tok::Ident("x".to_string())],
        vec![Tok::Ident("x".to_string()), Tok::RParen],
        vec![Tok::Ident("y".to_string()), Tok::Ident("z".to_string())],
        vec![Tok::RParen],
    ];
    for toks in &inputs {
        println!("{toks:?}: {:?}", parse_primary(toks));
    }
}
