//#![deny(missing_docs)]
//! A simple key/value store.

//pub use client::KvsClient;
pub use engines::{KvStore,KVEngine,SledStore};
pub use error::{KvsError, Result};
pub use server::KvServer;
pub use common::{Cmd,WrapCmd,parse_response};
pub use thread_pool::{ThreadPool,ShardThreadPool};
pub mod client;
pub mod common;

///a module represent kv engine
pub mod engines;
///a module about errors
pub mod error;
pub mod server;
pub mod thread_pool;
