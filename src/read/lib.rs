#[crate_id = "read#0.1.0"];
#[crate_type = "dylib"];
#[crate_type = "lib"];

#[feature(managed_boxes, globs, macro_registrar, macro_rules, quote)];

extern crate collections;
extern crate syntax;

pub mod parse;
pub mod macros;
pub mod buffer;
pub mod rt;

