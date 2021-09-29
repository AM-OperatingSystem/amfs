#![allow(dead_code)]
#![allow(clippy::unwrap_used)]
#![allow(missing_docs)]
#![allow(unknown_lints)]
#![allow(require_stability_comment)]

pub mod dg;
pub mod fsinit;

#[cfg(feature = "log4rs")]
pub mod logging;

#[cfg(not(feature = "log4rs"))]
pub mod logging {
    pub fn init_log() {}
}