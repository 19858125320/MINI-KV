use crate::Result;
pub mod native;
pub mod shard;
/// The trait that all thread pools should implement.
pub trait ThreadPool {
    /// Creates a new thread pool, immediately spawning the specified number of
    /// threads.
    ///
    /// Returns an error if any thread fails to spawn. All previously-spawned threads
    /// are terminated.
    fn new(threads: u32) -> Result<Self>
    where
        Self: Sized;

    /// Spawns a function into the thread pool.
    ///
    /// Spawning always succeeds, but if the function panics the threadpool continues
    /// to operate with the same number of threads &mdash; the thread count is not
    /// reduced nor is the thread pool destroyed, corrupted or invalidated.
    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static;

    fn stop(&mut self) -> Result<()>;
}

pub use native::NativeThreadPool;
pub use shard::ShardThreadPool;