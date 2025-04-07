use std::net::{SocketAddr, TcpListener, TcpStream};
use std::io::{self,BufReader, BufWriter, Write, Read};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use log::{debug, error, info};
use crate::{Cmd, KvsError, KVEngine,ThreadPool, Result};
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
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            }
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break, // 客户端关闭连接
            Err(e) => return Err(e).map_err(KvsError::Io),
        }
        let len = u32::from_be_bytes(len_buf) as usize;

        // 读取命令
        let mut command_buf = vec![0u8; len];
        reader.read_exact(&mut command_buf)?;
        let cmd=Cmd::decode(len as u32,command_buf)?;
        //info!("Received command: {:?}",cmd);
        match cmd{
            Cmd::Get(c)=>{
                info!("receive get cmd {:?} from client",c);
                let mut res=match engine.get(c.key){
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
                res.push('\n');
                writer.write_all(res.as_bytes())?;
            },
            Cmd::VGet(c)=>{
                info!("receive vget cmd {:?} from client",c);
                let mut res=match engine.get(c.key){
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
                res.push('\n');
                writer.write_all(res.as_bytes())?;
            }
            Cmd::Set(c)=>{
                info!("receive set cmd {:?}  from client",c);
                let mut res=match engine.set(c.key, c.value,c.expire){
                    Ok(_)=>{
                        let res=generate_response(true, "".to_string());
                        res
                    },
                    Err(e)=>{
                        let res=generate_response(false,format!("{}",e));
                        res
                    }
                };
                res.push('\n');
                writer.write_all(res.as_bytes())?;
            },
            Cmd::VSet(c)=>{
                info!("receive vset cmd {:?}  from client",c);
                let mut res=match engine.set(c.key, c.value,c.expire){
                    Ok(_)=>{
                        let res=generate_response(true, "".to_string());
                        res
                    },
                    Err(e)=>{
                        let res=generate_response(false,format!("{}",e));
                        res
                    }
                };
                res.push('\n');
                writer.write_all(res.as_bytes())?;
            }
            Cmd::Remove(c)=>{
                info!("receive remove cmd {:?}  from client",c);
                let mut res=match engine.remove(c.key){
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
                res.push('\n');
                writer.write_all(res.as_bytes())?;
            },
            Cmd::VDel(c)=>{
                info!("receive vdel cmd {:?}  from client",c);
                let mut res=match engine.remove(c.key){
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
                res.push('\n');
                writer.write_all(res.as_bytes())?;
            }
            Cmd::Scan(c)=>{
                info!("receive scan cmd {:?}  from client",c);
                let mut res=match engine.scan(c.start, c.end){
                    Ok(v)=>{
                        let mut res=String::from("OK");
                        let mut i=0;
                        let v_len=v.len();
                        for s in v{
                            res.push_str(&s);
                            if i!=v_len-1{
                                res.push(' ');
                            }
                            i+=1
                        }
                        res
                    },
                    Err(e)=>{
                        let res=generate_response(false,format!("{}",e));
                        res
                    }
                };
                res.push('\n');
                writer.write_all(res.as_bytes())?;
            }
            Cmd::Ping(c)=>{
                info!("receive ping cmd {:?}  from client",c);
                let mut res =generate_response(true, "PONG".to_string());
                if !c.message.is_empty(){
                    res=generate_response(true, c.message);
                }
                res.push('\n');
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
                    
                    self.pool.get_mut().spawn(move||{
                        handle_client(stream,addr,shutdown,store).unwrap();
                    });
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