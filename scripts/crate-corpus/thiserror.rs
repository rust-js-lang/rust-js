use std::error::Error as _;
use std::num::ParseIntError;

use thiserror::Error;

#[derive(Error, Debug)]
#[error("inner failure {code}")]
pub struct Inner {
    code: u32,
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid {field}: {reason}")]
    Invalid { field: &'static str, reason: String },
    #[error(transparent)]
    Parse(#[from] ParseIntError),
    #[error("wrapped")]
    Wrapped {
        #[source]
        inner: Inner,
    },
    #[error("{0:?} and {1:>4}")]
    Shown(Vec<u8>, u32),
}

fn parse(text: &str) -> Result<i32, AppError> {
    Ok(text.parse::<i32>()?)
}

pub fn report() -> String {
    let errors = [
        AppError::NotFound("user 7".into()),
        AppError::Invalid {
            field: "email",
            reason: "no @".into(),
        },
        parse("x1").unwrap_err(),
        AppError::Wrapped {
            inner: Inner { code: 42 },
        },
        AppError::Shown(vec![1, 2], 3),
    ];
    let mut lines = Vec::new();
    for e in &errors {
        let source = e.source().map(|s| s.to_string());
        lines.push(format!("{e} | {source:?}"));
    }
    lines.push(format!("{:?}", errors[0]));
    lines.push(format!("{:?}", parse("12").ok()));
    lines.join("\n")
}
