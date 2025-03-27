use std::thread;
use super::ThreadPool;
use crate::Result;
use crossbeam::channel::{self,Receiver,Sender};
use log::{debug, error, info};
// 定义任务类型：处理 TCP 连接
type Task = Box<dyn FnOnce() + Send + 'static>;

enum TaskMessage {
    NewTask(Task),
    Terminate,
}

struct Worker {
    id: u32,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: u32, receiver: Receiver<TaskMessage>) -> Worker {
        let thread = thread::spawn(move || 
        loop {
            let task = {
                let receiver = receiver.recv();
                match receiver {
                    Ok(task) => task,
                    Err(_) => {
                        debug!("Thread exits because the thread pool is destroyed.");
                        break; // 通道关闭，退出循环
                    }
                }
            };

            match task{
                TaskMessage::NewTask(t)=>{
                    info!("Worker {} receive new task, handle it.", id);
                     // 使用 catch_unwind 捕获 panic
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        t();
                    }));

                    if let Err(panic) = result {
                        error!("Worker {} caught panic: {:?}", id, panic);
                        // panic 后继续循环，不退出线程
                    }
                },
                TaskMessage::Terminate=>{
                    info!("Worker {} receive terminate task, exit.", id);
                    break;
                }
            }
           
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}

pub struct ShardThreadPool{
    workers:Vec<Worker>,
    sender:Sender<TaskMessage>,
}

impl ThreadPool for ShardThreadPool {
    fn new(_threads: u32) -> Result<Self> { 
        let (sender,receiver)=channel::unbounded::<TaskMessage>();
        let mut workers = Vec::with_capacity(_threads as usize);
        for i in 0.._threads{
            let receiver_clone=receiver.clone();
            let worker=Worker::new(i,receiver_clone);
            workers.push(worker);
        }
        Ok(Self {workers:workers,sender:sender})
     }
 
     fn spawn<F>(&self, job: F)
     where
         F: FnOnce() + Send + 'static,
     {
        self.sender.send(TaskMessage::NewTask(Box::new(job))).unwrap();
     }

     fn stop(&mut self) -> Result<()>{
        for _ in &self.workers {
            self.sender.send(TaskMessage::Terminate).unwrap();
        }
        for worker in &mut self.workers{
            if let Some(thread)=worker.thread.take(){
                thread.join().unwrap();
            }
        }
        Ok(())
     }
 }