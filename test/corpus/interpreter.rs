use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::iter::Peekable;
use std::rc::Rc;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Num(f64),
    Ident(String),
    Str(String),
    Op(char),
    LParen,
    RParen,
    Comma,
    Let,
    Fn,
    Arrow,
    Eq,
    Semi,
}

#[derive(Debug)]
enum LexError {
    Unexpected(char, usize),
    Unterminated,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LexError::Unexpected(c, at) => write!(f, "unexpected {:?} at {}", c, at),
            LexError::Unterminated => write!(f, "unterminated string"),
        }
    }
}

struct Lexer<'a> {
    chars: Peekable<Chars<'a>>,
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Lexer { chars: src.chars().peekable(), pos: 0 }
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        self.pos += 1;
        Some(c)
    }

    fn tokens(mut self) -> Result<Vec<Token>, LexError> {
        let mut out = Vec::new();
        while let Some(&c) = self.chars.peek() {
            if c.is_whitespace() {
                self.bump();
                continue;
            }
            if c.is_ascii_digit() {
                let mut text = String::new();
                while let Some(&d) = self.chars.peek() {
                    if d.is_ascii_digit() || d == '.' {
                        text.push(d);
                        self.bump();
                    } else {
                        break;
                    }
                }
                out.push(Token::Num(text.parse().unwrap()));
                continue;
            }
            if c.is_alphabetic() || c == '_' {
                let mut name = String::new();
                while let Some(&d) = self.chars.peek() {
                    if d.is_alphanumeric() || d == '_' {
                        name.push(d);
                        self.bump();
                    } else {
                        break;
                    }
                }
                out.push(match name.as_str() {
                    "let" => Token::Let,
                    "fn" => Token::Fn,
                    _ => Token::Ident(name),
                });
                continue;
            }
            let at = self.pos;
            self.bump();
            let token = match c {
                '"' => {
                    let mut s = String::new();
                    loop {
                        match self.bump() {
                            Some('"') => break,
                            Some(ch) => s.push(ch),
                            None => return Err(LexError::Unterminated),
                        }
                    }
                    Token::Str(s)
                }
                '(' => Token::LParen,
                ')' => Token::RParen,
                ',' => Token::Comma,
                ';' => Token::Semi,
                '=' if self.chars.peek() == Some(&'>') => {
                    self.bump();
                    Token::Arrow
                }
                '=' => Token::Eq,
                '+' | '-' | '*' | '/' | '<' => Token::Op(c),
                other => return Err(LexError::Unexpected(other, at)),
            };
            out.push(token);
        }
        Ok(out)
    }
}

#[derive(Debug)]
enum Expr {
    Num(f64),
    Str(String),
    Var(String),
    Binary(char, Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Lambda(Rc<Vec<String>>, Rc<Expr>),
}

enum Stmt {
    Let(String, Expr),
    Expr(Expr),
}

struct Parser {
    tokens: Vec<Token>,
    at: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.at)
    }

    fn next(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.at).cloned();
        self.at += 1;
        t
    }

    fn expect(&mut self, want: Token) -> Result<(), String> {
        match self.next() {
            Some(t) if t == want => Ok(()),
            Some(t) => Err(format!("expected {:?}, found {:?}", want, t)),
            None => Err(format!("expected {:?}, found the end", want)),
        }
    }

    fn program(&mut self) -> Result<Vec<Stmt>, String> {
        let mut stmts = vec![];
        while self.peek().is_some() {
            let stmt = if self.peek() == Some(&Token::Let) {
                self.next();
                let name = match self.next() {
                    Some(Token::Ident(n)) => n,
                    t => return Err(format!("expected a name, found {:?}", t)),
                };
                self.expect(Token::Eq)?;
                Stmt::Let(name, self.expr()?)
            } else {
                Stmt::Expr(self.expr()?)
            };
            stmts.push(stmt);
            if self.peek().is_some() {
                self.expect(Token::Semi)?;
            }
        }
        Ok(stmts)
    }

    fn precedence(op: char) -> u8 {
        match op {
            '<' => 1,
            '+' | '-' => 2,
            _ => 3,
        }
    }

    fn expr(&mut self) -> Result<Expr, String> {
        self.binary(0)
    }

    fn binary(&mut self, min: u8) -> Result<Expr, String> {
        let mut left = self.call()?;
        while let Some(&Token::Op(op)) = self.peek() {
            let p = Self::precedence(op);
            if p <= min {
                break;
            }
            self.next();
            let right = self.binary(p)?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn call(&mut self) -> Result<Expr, String> {
        let mut e = self.atom()?;
        while self.peek() == Some(&Token::LParen) {
            self.next();
            let mut args = Vec::new();
            if self.peek() != Some(&Token::RParen) {
                loop {
                    args.push(self.expr()?);
                    if self.peek() == Some(&Token::Comma) {
                        self.next();
                    } else {
                        break;
                    }
                }
            }
            self.expect(Token::RParen)?;
            e = Expr::Call(Box::new(e), args);
        }
        Ok(e)
    }

    fn atom(&mut self) -> Result<Expr, String> {
        match self.next() {
            Some(Token::Num(n)) => Ok(Expr::Num(n)),
            Some(Token::Str(s)) => Ok(Expr::Str(s)),
            Some(Token::Ident(n)) => Ok(Expr::Var(n)),
            Some(Token::LParen) => {
                let e = self.expr()?;
                self.expect(Token::RParen)?;
                Ok(e)
            }
            Some(Token::Fn) => {
                self.expect(Token::LParen)?;
                let mut params = vec![];
                while let Some(Token::Ident(p)) = self.peek().cloned() {
                    params.push(p);
                    self.next();
                    if self.peek() == Some(&Token::Comma) {
                        self.next();
                    }
                }
                self.expect(Token::RParen)?;
                self.expect(Token::Arrow)?;
                let body = self.expr()?;
                Ok(Expr::Lambda(Rc::new(params), Rc::new(body)))
            }
            t => Err(format!("unexpected {:?}", t)),
        }
    }
}

#[derive(Clone)]
enum Value {
    Num(f64),
    Str(Rc<str>),
    Bool(bool),
    Closure(Rc<Vec<String>>, Rc<Expr>, Env),
    Builtin(&'static str, fn(&[Value]) -> Result<Value, String>),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Num(n) => write!(f, "{}", n),
            Value::Str(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Closure(params, ..) => write!(f, "<fn({})>", params.join(", ")),
            Value::Builtin(name, _) => write!(f, "<builtin {}>", name),
        }
    }
}

#[derive(Clone, Default)]
struct Env(Rc<RefCell<Scope>>);

#[derive(Default)]
struct Scope {
    vars: HashMap<String, Value>,
    parent: Option<Env>,
}

impl Env {
    fn child(&self) -> Env {
        Env(Rc::new(RefCell::new(Scope { vars: HashMap::new(), parent: Some(self.clone()) })))
    }

    fn get(&self, name: &str) -> Option<Value> {
        let scope = self.0.borrow();
        match scope.vars.get(name) {
            Some(v) => Some(v.clone()),
            None => scope.parent.as_ref().and_then(|p| p.get(name)),
        }
    }

    fn set(&self, name: &str, value: Value) {
        self.0.borrow_mut().vars.insert(name.to_string(), value);
    }
}

fn len(args: &[Value]) -> Result<Value, String> {
    match args {
        [Value::Str(s)] => Ok(Value::Num(s.chars().count() as f64)),
        _ => Err("len takes one string".to_string()),
    }
}

fn upper(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::Str(s)) => Ok(Value::Str(s.to_uppercase().into())),
        Some(other) => Err(format!("upper of {}", other)),
        None => Err("upper of nothing".into()),
    }
}

struct Interp {
    depth: usize,
    calls: usize,
}

impl Interp {
    fn eval(&mut self, e: &Expr, env: &Env) -> Result<Value, String> {
        Ok(match e {
            Expr::Num(n) => Value::Num(*n),
            Expr::Str(s) => Value::Str(Rc::from(s.as_str())),
            Expr::Var(name) => env.get(name).ok_or_else(|| format!("unbound {}", name))?,
            Expr::Binary(op, l, r) => {
                let l = self.eval(l, env)?;
                let r = self.eval(r, env)?;
                match (op, l, r) {
                    ('+', Value::Num(a), Value::Num(b)) => Value::Num(a + b),
                    ('+', Value::Str(a), b) => Value::Str(format!("{}{}", a, b).into()),
                    ('-', Value::Num(a), Value::Num(b)) => Value::Num(a - b),
                    ('*', Value::Num(a), Value::Num(b)) => Value::Num(a * b),
                    ('/', Value::Num(_), Value::Num(b)) if b == 0.0 => return Err("division by zero".into()),
                    ('/', Value::Num(a), Value::Num(b)) => Value::Num(a / b),
                    ('<', Value::Num(a), Value::Num(b)) => Value::Bool(a < b),
                    (op, a, b) => return Err(format!("cannot {} {} and {}", op, a, b)),
                }
            }
            Expr::Lambda(params, body) => Value::Closure(params.clone(), body.clone(), env.clone()),
            Expr::Call(callee, args) => {
                let f = self.eval(callee, env)?;
                let args = args.iter().map(|a| self.eval(a, env)).collect::<Result<Vec<_>, _>>()?;
                self.calls += 1;
                match f {
                    Value::Builtin(_, f) => f(&args)?,
                    Value::Closure(params, body, captured) => {
                        if params.len() != args.len() {
                            return Err(format!("expected {} arguments, got {}", params.len(), args.len()));
                        }
                        let scope = captured.child();
                        for (p, a) in params.iter().zip(args) {
                            scope.set(p, a);
                        }
                        self.depth += 1;
                        let result = self.eval(&body, &scope);
                        self.depth -= 1;
                        result?
                    }
                    other => return Err(format!("{} is not a function", other)),
                }
            }
        })
    }
}

fn run(src: &str) -> Result<Vec<String>, String> {
    let tokens = Lexer::new(src).tokens().map_err(|e| e.to_string())?;
    let program = Parser { tokens, at: 0 }.program()?;
    let globals = Env::default();
    globals.set("len", Value::Builtin("len", len));
    globals.set("upper", Value::Builtin("upper", upper));
    let mut interp = Interp { depth: 0, calls: 0 };
    let mut out = vec![];
    for stmt in &program {
        match stmt {
            Stmt::Let(name, e) => {
                let v = interp.eval(e, &globals)?;
                globals.set(name, v);
            }
            Stmt::Expr(e) => out.push(interp.eval(e, &globals)?.to_string()),
        }
    }
    out.push(format!("calls={} depth={}", interp.calls, interp.depth));
    Ok(out)
}

fn main() {
    let programs = [
        "1 + 2 * 3; (1 + 2) * 3; 10 / 4; 2 < 3",
        "let add = fn(a, b) => a + b; add(2, 3); add",
        "let k = fn(x) => fn(y) => x * y; let twice = k(2); twice(21); k(3)(4)",
        "let greet = fn(n) => \"hi \" + upper(n); greet(\"ann\"); len(\"héllo\")",
        "1 / 0",
        "let f = fn(x) => x; f(1, 2)",
        "nope(1)",
        "\"abc",
        "1 + #",
        "let = 3",
        "(1 + 2",
        "3(4)",
        "upper(5)",
        "\"n=\" + 1.5",
        "let fact = fn(n) => n * 1; fact(5); fact",
    ];
    for (i, src) in programs.iter().enumerate() {
        match run(src) {
            Ok(lines) => println!("{}: {}", i, lines.join(" | ")),
            Err(e) => println!("{}: error: {}", i, e),
        }
    }
    let tokens = Lexer::new("let x = fn(a) => a;").tokens().unwrap();
    println!("{:?}", tokens);
    let e = Parser { tokens: Lexer::new("f(1, \"s\") + 2").tokens().unwrap(), at: 0 }.expr().unwrap();
    println!("{:?}", e);
}
