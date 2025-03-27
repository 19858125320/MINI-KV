use std::thread;
use super::ThreadPool;
use crate::Result;
pub struct NativeThreadPool;

impl ThreadPool for NativeThreadPool {
   fn new(_threads: u32) -> Result<Self> { 
        Ok(NativeThreadPool)
    }

    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        thread::spawn(job);
    }

    fn stop(&mut self) -> Result<()>{
        Ok(())
    }
}
