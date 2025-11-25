#![cfg_attr(doc,doc = document_features::document_features!())]

pub mod io;
pub mod parsesource;
pub mod sourcerefs;
pub mod strings;

pub use {
    parsesource::ParseSource,
    sourcerefs::{LSPLineCol, SourcePos, SourceRange},
};
/*
#[cfg(not(feature = "serde"))]
pub trait CondSerialize {}
#[cfg(not(feature = "serde"))]
impl<T> CondSerialize for T {}

#[cfg(feature = "serde")]
pub trait CondSerialize: serde::Serialize + for<'de> serde::Deserialize<'de> {}

#[cfg(feature = "serde")]
impl<T: serde::Serialize + for<'de> serde::Deserialize<'de>> CondSerialize for T {}
 */
