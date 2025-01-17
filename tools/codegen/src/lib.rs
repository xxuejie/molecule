#![recursion_limit = "256"]

pub(crate) mod ast;
pub(crate) mod compiler;
pub(crate) mod generator;
pub(crate) mod parser;
pub(crate) mod utils;

pub(crate) use ast::verified::Ast;
pub use compiler::Compiler;
pub(crate) use generator::Generator;
pub use generator::Language;
pub(crate) use parser::Parser;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
