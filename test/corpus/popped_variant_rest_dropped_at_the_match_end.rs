// What's left of a popped enum one arm of a `match` moved a part of one
// variant out of, and another arm a part of another variant, dropped where
// the `match` ends, as regex-syntax's `ParserI::pop_group_end` pops its
// stack: each variant's rest by the enum's own state, under the test of
// its discriminant, as rustc's drop ladder drops it (#2177).
use std::cell::RefCell;

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

enum GroupState {
    Group {
        concat: Loud,
        group: Loud,
        ignore_whitespace: bool,
    },
    Alternation(Loud),
}

struct Parser {
    stack: RefCell<Vec<GroupState>>,
}

impl Parser {
    fn pop_group_end(&self, concat: Loud) -> Result<Loud, &'static str> {
        let mut stack = self.stack.borrow_mut();
        let ast = match stack.pop() {
            None => Ok(concat),
            Some(GroupState::Alternation(alt)) => {
                println!("alt {} after {}", alt.0, concat.0);
                Ok(alt)
            }
            Some(GroupState::Group { group, .. }) => {
                return Err(group.0);
            }
        };
        match stack.pop() {
            None => ast,
            Some(GroupState::Alternation(_)) => Err("two alternations"),
            Some(GroupState::Group {
                group,
                ignore_whitespace,
                ..
            }) => {
                println!("unclosed {} {ignore_whitespace}", group.0);
                Err(group.0)
            }
        }
    }
}

fn run(name: &str, stack: Vec<GroupState>) {
    let parser = Parser {
        stack: RefCell::new(stack),
    };
    match parser.pop_group_end(Loud("concat")) {
        Ok(ast) => println!("{name}: ok {}", ast.0),
        Err(e) => println!("{name}: err {e}"),
    }
    for state in parser.stack.borrow().iter() {
        match state {
            GroupState::Group { concat, .. } => println!("{name}: left a group after {}", concat.0),
            GroupState::Alternation(alt) => println!("{name}: left alternation {}", alt.0),
        }
    }
}

fn group(name: &'static str) -> GroupState {
    GroupState::Group {
        concat: Loud(name),
        group: Loud("g"),
        ignore_whitespace: true,
    }
}

fn main() {
    run("empty", vec![]);
    run("alt", vec![GroupState::Alternation(Loud("a"))]);
    run("group", vec![group("c1")]);
    run("group then alt", vec![group("c2"), GroupState::Alternation(Loud("a2"))]);
    run(
        "two alts",
        vec![GroupState::Alternation(Loud("x")), GroupState::Alternation(Loud("y"))],
    );
    run("alt under group", vec![GroupState::Alternation(Loud("u")), group("c3")]);
    run("groups", vec![group("c4"), group("c5")]);
}
