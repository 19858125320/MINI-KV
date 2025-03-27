use crate::Result;

///KVEngine is a abstract interface
pub trait KVEngine:Clone+Send + 'static{
    ///set key value string to kv engine
    fn set(&self, key: String, value: String) -> Result<()>;

    ///get value string from kv engine
    fn get(&self, key: String) -> Result<Option<String>>;

    ///remove key value string from kv engine
    fn remove(&self, key: String) -> Result<()>;
}

mod kvs;
mod sled;

pub use self::kvs::KvStore;
pub use self::sled::SledStore;