// A type's own `FromStr`: what `s.parse::<T>()` calls, and `T::from_str(s)`,
// as shared models read what a form or a URL holds.

use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Role {
    Admin,
    Member,
}

impl FromStr for Role {
    type Err = String;

    fn from_str(s: &str) -> Result<Role, String> {
        match s.trim().to_lowercase().as_str() {
            "admin" => Ok(Role::Admin),
            "member" => Ok(Role::Member),
            other => Err(format!("no role {other:?}")),
        }
    }
}

#[derive(Debug, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Debug)]
enum PointError {
    Shape,
    Number(std::num::ParseIntError),
}

impl fmt::Display for PointError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PointError::Shape => write!(f, "expected x,y"),
            PointError::Number(e) => write!(f, "bad number: {e}"),
        }
    }
}

impl FromStr for Point {
    type Err = PointError;

    fn from_str(s: &str) -> Result<Point, PointError> {
        let (x, y) = s.split_once(',').ok_or(PointError::Shape)?;
        let x = x.trim().parse().map_err(PointError::Number)?;
        let y = y.trim().parse().map_err(PointError::Number)?;
        Ok(Point { x, y })
    }
}

fn total(text: &str) -> Result<i32, PointError> {
    let mut sum = 0;
    for part in text.split(';') {
        let p: Point = part.parse()?;
        sum += p.x + p.y;
    }
    Ok(sum)
}

fn main() {
    println!("{:?} {:?} {:?}", "Admin".parse::<Role>(), " member ".parse::<Role>(), "guest".parse::<Role>());
    println!("{:?}", Role::from_str("ADMIN").map(|r| r == Role::Admin));
    let roles: Result<Vec<Role>, String> = ["admin", "member"].iter().map(|s| s.parse()).collect();
    println!("{:?}", roles);

    println!("{:?}", "3, 4".parse::<Point>());
    match "3".parse::<Point>() {
        Ok(p) => println!("{p:?}"),
        Err(e) => println!("{e}"),
    }
    if let Err(e) = "3,x".parse::<Point>() {
        println!("{e} {e:?}");
    }
    println!("{:?}", total("1,2; 3,4").ok());
    println!("{}", total("1,2;oops").map_or_else(|e| e.to_string(), |n| n.to_string()));
}
