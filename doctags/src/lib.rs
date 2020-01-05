#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

pub mod config;
pub mod doctags;
pub mod index;
pub mod search;
pub mod vfs;
pub mod walk;

pub use tantivy::Index;
