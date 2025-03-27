use super::KVEngine;
use crate::{KvsError, Result};
use sled::{self,Db};
use std::path::PathBuf;

#[derive(Clone)]
pub struct SledStore{
    t: Db,
}

impl SledStore{
    pub fn open(path: impl Into<PathBuf>)->Result<Self>{
        let db=sled::open(path.into())?;
        Ok(Self{t:db})
    }
}

impl KVEngine for SledStore{
    fn set(&self, key: String, value: String) -> Result<()> {
        self.t.insert(key.as_bytes(),value.as_bytes())?;
        self.t.flush()?;
        Ok(())
    }

    fn get(&self, key: String) -> Result<Option<String>> {
        let res=self.t.get(key.as_bytes())?;
        match res{
            Some(v)=>{
                let s=String::from_utf8(v.to_vec())?;
                Ok(Some(s))
            },
            None=>Ok(None),
        }
    }

    fn remove(&self, key: String) -> Result<()> {
        let res=self.t.remove(key.as_bytes())?;
        if res.is_none(){
            return Err(KvsError::KeyNotFound);
        }
        self.t.flush()?;
        Ok(())
    }
}