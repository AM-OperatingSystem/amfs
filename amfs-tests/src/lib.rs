#![allow(unknown_lints)]
#![allow(require_stability_comment)]

#[cfg(not(tarpaulin_include))]
pub mod imagegen;
#[cfg(not(tarpaulin_include))]
pub mod logging;

#[macro_use]
extern crate lazy_static;