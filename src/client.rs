//use std::fmt::Result;
use std::net::{TcpStream, SocketAddr};
use std::io::{BufReader, BufWriter, Write, Read};
use crate::{Result,parse_response, WrapCmd};

pub struct KvClient{
    reader: BufReader<TcpStream>,
    writer: BufWriter<TcpStream>,
}

impl KvClient{
    pub fn new(addr:SocketAddr)->Result<Self>{
        let stream = TcpStream::connect(addr)?;
        let reader = BufReader::new(stream.try_clone()?); // 克隆流用于读取
        let writer = BufWriter::new(stream);              // 原流用于写入
        //info!("Connected to server at {}", addr);
        Ok(KvClient { reader, writer })
    }

    pub fn send_request(&mut self,cmd:WrapCmd)->Result<String>{
        let buf=cmd.encode();
        self.writer.write_all(buf.as_slice())?;
        self.writer.flush()?;
        //info!("send request {:?} to server",cmd);

        //读取响应
        let mut response=String::new();
        self.reader.read_to_string(&mut response)?;
        let res=parse_response(response)?;
        Ok(res)
    }
}