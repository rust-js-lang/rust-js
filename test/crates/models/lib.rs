//! The crate in the middle (ADR 0100): types, their derives and methods,
//! and one of each thing a consumer can't see from the types alone.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Role {
    Admin,
    Member,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub name: String,
    pub email: String,
    pub age: u32,
    pub role: Role,
}

impl User {
    pub fn new(name: &str, email: &str, age: u32) -> User {
        User { name: name.to_string(), email: email.to_string(), age, role: Role::Member }
    }

    pub fn validate(&self) -> Result<(), String> {
        validation::email(&self.email)?;
        if !validation::between(self.age, 13, 130) {
            return Err(format!("{} is {}, too young", self.name, self.age));
        }
        Ok(())
    }

    /// Changes a `User` in place, which `frontend` never does itself.
    pub fn birthday(&mut self) {
        self.age += 1;
    }
}

/// Read from JSON and changed in place by `frontend` alone, never by
/// `models`: its codec is `models`' all the same (#341).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    pub text: String,
    pub tags: Vec<String>,
}

/// A `Copy` type, and a constant of it that `origin` hands out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

pub const ORIGIN: Point = Point { x: 0, y: 0 };

pub fn origin() -> Point {
    ORIGIN
}

/// An address compared as its lowercase text, by hand: `==` of one isn't
/// its fields'.
#[derive(Debug, Clone)]
pub struct Email(pub String);

impl PartialEq for Email {
    fn eq(&self, other: &Email) -> bool {
        self.0.to_lowercase() == other.0.to_lowercase()
    }
}

/// Takes what it's given, and drops it at its end: a consumer's value with
/// a destructor too, a type `models` never sees.
pub fn count<T>(items: Vec<T>) -> usize {
    items.len()
}

/// What `models` changes in place through its own method: its `Vec`, which
/// `frontend` never takes `&mut` of itself.
#[derive(Debug, Clone, Default)]
pub struct Bag {
    pub items: Vec<u32>,
}

impl Bag {
    pub fn add(&mut self, n: u32) {
        self.items.push(n);
    }
}

/// A temperature written by hand as one string, `"21.5C"`, as serde's own
/// guide writes a type it can't derive: its `Serialize` and its
/// `Deserialize` with a `Visitor` of its own, each generic over serde's
/// `Serializer` or `Deserializer`, which a consumer compiles itself (ADR
/// 0914, #1560).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Celsius(pub f64);

impl Serialize for Celsius {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{}C", self.0))
    }
}

struct CelsiusVisitor;

impl serde::de::Visitor<'_> for CelsiusVisitor {
    type Value = Celsius;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("a temperature like 21.5C")
    }

    fn visit_str<E: serde::de::Error>(self, text: &str) -> Result<Celsius, E> {
        let degrees = text.strip_suffix('C').ok_or_else(|| E::custom("no C"))?;
        degrees.parse().map(Celsius).map_err(E::custom)
    }
}

impl<'de> Deserialize<'de> for Celsius {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(CelsiusVisitor)
    }
}
