//! The package named `chrono`, re-exporting IronChrono for `[patch.crates-io]`.
//!
//! Everything public in IronChrono is public here under the same path, and the
//! types are the same types: `chrono::DateTime<Utc>` from this package and from
//! `ironchrono` are one type.
#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[doc(inline)]
pub use iron::*;
