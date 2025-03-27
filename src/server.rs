use std::net::{SocketAddr, TcpListener, TcpStream};
use std::io::{self,BufReader, BufWriter, Write, Read};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use log::{debug, error, info};
use crate::{Cmd, KvsError, KVEngine,ThreadPool, Result, WrapCmd};
use std::cell::RefCell;

pub struct KvServer<E:KVEngine,P:ThreadPool>{
    engine:E,
    listener:TcpListener,
    shut_down:Arc<AtomicBool>,
    pool:RefCell<P>,
}

fn generate_response(success:bool,s:String)->String{
    if success{
        format!("OK{}",s)
    }else{
        format!("Error{}",s)
    }
}

fn handle_client<E:KVEngine>(stream:TcpStream,peer_addr:SocketAddr,shut_down:Arc<AtomicBool>,engine:E)->Result<()>{
    let mut reader=BufReader::new(stream.try_clone()?);
    let mut writer=BufWriter::new(stream);
    
    let shutdown=shut_down.clone();
    loop {
        if shutdown.load(Ordering::SeqCst) {
            debug!("Shutting down client handler for {}", peer_addr);
            break;
        }

        // 读取 4 字节长度
        let mut len_buf = [0u8; 4];
        match reader.read_exact(&mut len_buf) {
            Ok(()) => (),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                //std::thread::sleep(std::time::Duration::from_millis(10));
                break;
            }
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break, // 客户端关闭连接
            Err(e) => return Err(e).map_err(KvsError::Io),
        }
        let len = u32::from_be_bytes(len_buf) as usize;

        // 读取命令
        let mut command_buf = vec![0u8; len];
        reader.read_exact(&mut command_buf)?;
        let cmd=WrapCmd::decode(len as u32,command_buf)?;
        info!("Received command: {:?}",cmd);
        match cmd.cmd{
            Cmd::Get(_)=>{
                info!("receive get cmd {:?} from client",cmd);
                let res=match engine.get(cmd.key){
                    Ok(Some(v))=>{
                        let res=generate_response(true, v);
                        res
                    },
                    Ok(None)=>{
                        let res=generate_response(false,"Key not found".to_string());
                        res
                    },
                    Err(e)=>{
                        let res=generate_response(false,format!("{}",e));
                        res
                    }
                };
                writer.write_all(res.as_bytes())?;
            },
            Cmd::Set(_)=>{
                info!("receive set cmd {:?}  from client",cmd);
                let res=match engine.set(cmd.key, cmd.value){
                    Ok(_)=>{
                        let res=generate_response(true, "".to_string());
                        res
                    },
                    Err(e)=>{
                        let res=generate_response(false,format!("{}",e));
                        res
                    }
                };
                writer.write_all(res.as_bytes())?;
            },
            Cmd::Remove(_)=>{
                info!("receive remove cmd {:?}  from client",cmd);
                let res=match engine.remove(cmd.key){
                    Ok(_)=>{
                        let res=generate_response(true, "".to_string());
                        res
                    },
                    Err(KvsError::KeyNotFound)=>{
                        let res=generate_response(false,"Key not found".to_string());
                        res
                    }
                    Err(e)=>{
                        let res=generate_response(false,format!("{}",e));
                        res
                    }
                };
                writer.write_all(res.as_bytes())?;
            }
        }
        
        writer.flush()?;
    }

    info!("Client {} disconnected", peer_addr);
    Ok(())
}

impl<E:KVEngine,P:ThreadPool> KvServer<E,P>{
    pub fn new(engine:E,addr:SocketAddr,shut_down:Arc<AtomicBool>,pool:P)->Result<Self>{
        let listener=TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        Ok(KvServer{engine,listener,shut_down,pool:RefCell::new(pool)})
    }

    pub fn run(&mut self)->Result<()>{
        loop {
            if self.shut_down.load(Ordering::SeqCst) {
                info!("Shutdown!Stopping accepting new connections...");

                break;
            }
            
            match self.listener.accept() {
                Ok((stream, addr)) => {
                    info!("accept connection:{:?}",addr);
                    let store = self.engine.clone();
                    let shutdown = self.shut_down.clone();
                    //let stream=stream.try_clone()?;
                    self.pool.get_mut().spawn(move||{
                        //let mut store=Arc::new(KvStore::open(std::path::Path::new("."))?);
                        handle_client(stream,addr,shutdown,store).unwrap();
                    });
                    //self.handle_client(stream,addr)?;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                }
                Err(e) => {
                    error!("Error accepting connection: {}", e);
                    break;
                }
            }
        }
        
        Ok(())
    }

    pub fn shut_down(&mut self){
        self.pool.get_mut().stop().unwrap();
    }
}