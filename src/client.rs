use std::net::SocketAddr;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{tcp::{OwnedReadHalf,OwnedWriteHalf},TcpStream};
use tokio::time::{self,Duration};
use crate::{Result,parse_response, WrapCmd};
use log::{error,info, warn};

pub struct KvClient{
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
}

impl KvClient{
    pub async fn new(addr:SocketAddr)->Result<Self>{
        let mut attempts = 0;
        //连接重试
        loop{
            match TcpStream::connect(addr).await{
                Ok(stream)=>{
                    info!("Connected to server success");
                    let (reader, writer) = stream.into_split();
                    return Ok(KvClient { reader: BufReader::new(reader), writer });
                },
                Err(e)=>{
                    attempts += 1;
                    if attempts >= 5 {
                        error!("Failed to connect to server at {}: {}", addr, e);
                        return Err(e.into());
                    }
                    warn!("Failed to connect to server at {}: {}. Retrying ({}/5)...", addr, e, attempts);
                    time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    pub async fn send_request(&mut self,cmd:WrapCmd)->Result<String>{
        let buf=cmd.encode();
        self.writer.write_all(buf.as_slice()).await?;
        self.writer.flush().await?;
        info!("send request {:?} to server",cmd);

        //读取响应
        let mut response=String::new();
        self.reader.read_line(&mut response).await?;
        let res=parse_response(response).await?;
        
        Ok(res)
    }
}