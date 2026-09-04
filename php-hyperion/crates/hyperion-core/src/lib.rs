pub mod memory;
pub mod gc;
pub mod types;
pub mod error;
#[macro_use]
pub mod macros;

use std::sync::atomic::AtomicU64;
pub static DB_MUTATION_VERSION: AtomicU64 = AtomicU64::new(0);
