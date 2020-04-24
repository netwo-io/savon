#[macro_use]
extern crate log;
extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate quote;

pub mod gen;
pub mod rpser;
pub mod wsdl;
pub mod http;
mod error;
pub use error::*;
