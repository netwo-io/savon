#[macro_use]
extern crate log;
extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate quote;

pub mod gen;
//mod rpser;
//mod http;
pub mod wsdl;

pub enum Error {}
