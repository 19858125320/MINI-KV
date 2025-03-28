use std::fs::{self,OpenOptions};
use std::path::{Path, PathBuf};
use env_logger::Builder;
use std::io::Write;
use crate::{Result,KvsError};
use regex::Regex;
//请求协议格式
/* 
  4     1     4              4              4
<len><cmd><keylen><key>[<valuelen><value>]<ttl> //其中value部分只有set命令才有
*/

#[derive(Copy,Clone,Debug,PartialEq,Eq)]
pub enum Cmd{
    Get(u8),
    Set(u8),
    Remove(u8),
    Scan(u8),
    //以下是向量类型相关的命令
    VGet(u8),
    VSet(u8),
    VDel(u8),
}

impl Cmd{
    fn from_u8(c:u8)->Self{
        if c==1{
            return Cmd::Get(1);
        }else if c==2{
            return Cmd::Set(2);
        }else if c==3{
            return Cmd::Remove(3);
        }else if c==4{
            return Cmd::Scan(4);
        }else if c==5{
            return Cmd::VGet(5);
        }else if c==6{
            return Cmd::VSet(6);
        }else if c==7{
            return Cmd::VDel(7);
        }
        panic!("Invalid Cmd");
    }

    pub fn to_string(&self)->String{
        match self{
            Cmd::Get(_)=>"Get".to_string(),
            Cmd::Set(_)=>"Set".to_string(),
            Cmd::Remove(_)=>"Remove".to_string(),
            Cmd::Scan(_)=>"Scan".to_string(),
            Cmd::VGet(_)=>"VGet".to_string(),
            Cmd::VSet(_)=>"VSet".to_string(),
            Cmd::VDel(_)=>"VDel".to_string(),
        }
    }
}
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct WrapCmd {
    pub cmd:Cmd,
    pub key:String,
    pub value:String,
    pub expire:u32,
}

impl WrapCmd{
    pub fn new_extra(cmd:Cmd,key:String,val:String,ttl:u32)->Self{
        Self{cmd:cmd,key:key,value:val,expire:ttl}
    }

    pub fn encode(&self)->Vec<u8>{
        let mut res=Vec::new();
        let mut fres=Vec::new();
        let mut len:u32=1;
        match self.cmd{
            Cmd::Get(c)|Cmd::VGet(c)=>{
                res.push(c);
                len+=4;
                len+=self.key.len() as u32;
                res.extend(u32::to_be_bytes(self.key.len() as u32));
                res.extend_from_slice(self.key.as_bytes());
            },
            Cmd::Set(c)|Cmd::VSet(c)=>{
                res.push(c);
                len+=12;
                len+=self.key.len() as u32;
                len+=self.value.len() as u32;
                res.extend(u32::to_be_bytes(self.key.len() as u32));
                res.extend_from_slice(self.key.as_bytes());
                res.extend(u32::to_be_bytes(self.value.len() as u32));
                res.extend_from_slice(self.value.as_bytes());
                res.extend(u32::to_be_bytes(self.expire));
            },
            Cmd::Remove(c)=>{
                res.push(c);
                len+=4;
                len+=self.key.len() as u32;
                res.extend(u32::to_be_bytes(self.key.len() as u32));
                res.extend_from_slice(self.key.as_bytes());
            },
            Cmd::VDel(c)=>{
                res.push(c);
                len+=4;
                len+=self.key.len() as u32;
                res.extend(u32::to_be_bytes(self.key.len() as u32));
                res.extend_from_slice(self.key.as_bytes());
            },
            Cmd::Scan(c)=>{
                res.push(c);
                len+=8;
                len+=self.key.len() as u32;
                len+=self.value.len() as u32;
                res.extend(u32::to_be_bytes(self.key.len() as u32));
                res.extend_from_slice(self.key.as_bytes());
                res.extend(u32::to_be_bytes(self.value.len() as u32));
                res.extend_from_slice(self.value.as_bytes());
            }
        }
        fres.extend(u32::to_be_bytes(len));
        fres.extend_from_slice(res.as_slice());
        fres
    }

    pub fn decode(len:u32,s:Vec<u8>)->Result<Self>{
        //提取长度
        // let bytes:[u8;4]=s[0..4].try_into().unwrap();
        // let len=u32::from_be_bytes(bytes);
        if len> s.len() as u32{
            return Err(KvsError::DecodeError);
        }

        //解析cmd
        let cmd=Cmd::from_u8(s[0]);

        //解析key
        let bytes:[u8;4]=s[1..5].try_into().unwrap();
        let key_len=u32::from_be_bytes(bytes);
        let key=String::from_utf8(s[5..5+key_len as usize].to_vec()).unwrap();

        let mut res=WrapCmd{
            cmd:cmd,
            key:key,
            value:String::new(),
            expire:0,
        };
        //如果有value,解析value
        if let Cmd::Set(_)=cmd {
            let st=5+key_len as usize;
            let bytes:[u8;4]=s[st..st+4].try_into().unwrap();
            let val_len=u32::from_be_bytes(bytes);
            let val=String::from_utf8(s[st+4..st+4+val_len as usize].to_vec()).unwrap();
            res.value=val;

            let st=st+4+val_len as usize;
            let bytes:[u8;4]=s[st..st+4].try_into().unwrap();
            let ttl=u32::from_be_bytes(bytes);
            res.expire=ttl;
        }else if let Cmd::VSet(_)=cmd{
            let st=5+key_len as usize;
            let bytes:[u8;4]=s[st..st+4].try_into().unwrap();
            let val_len=u32::from_be_bytes(bytes);
            let val=String::from_utf8(s[st+4..st+4+val_len as usize].to_vec()).unwrap();
            res.value=val;
        }else if let Cmd::Scan(_)=cmd{
            let st=5+key_len as usize;
            let bytes:[u8;4]=s[st..st+4].try_into().unwrap();
            let val_len=u32::from_be_bytes(bytes);
            let val=String::from_utf8(s[st+4..st+4+val_len as usize].to_vec()).unwrap();
            res.value=val;
        }
        Ok(res)
    }
}


//响应协议格式
/*
成功：OK[value]..[value]\n//只有Get响应有value,scan可能会有多个以空格间隔的value
失败：Error<message>\n
*/

pub async fn parse_response(s:String)->Result<String>{
    let len=s.len();
    if s.starts_with("OK"){
        if len>3{
            return Ok(s[2..len-1].to_string());
        }
        return Ok("".to_string());
    }else{
        let message=s[5..].to_string();
        if message.trim()=="Key not found"{
            return Err(KvsError::KeyNotFound);
        }
        return Err(KvsError::StringError(message));
    }
}

pub fn init_logger(log_dir: &str,is_client:bool) -> Result<()> {
    // 确保日志目录存在
    fs::create_dir_all(log_dir)?;

    let log_path:PathBuf;
    // 构造日志文件路径
    if is_client{
        log_path = Path::new(log_dir).join("kvs-client.log");
    }else{
        log_path = Path::new(log_dir).join("kvs-server.log");
    }
    let log_file = OpenOptions::new()
    .write(true)
    .append(true)
    .create(true)
    .open(log_path)?;

    // 配置 env_logger
    Builder::new()
        .filter_level(log::LevelFilter::Info) // 设置日志级别
        .target(env_logger::Target::Pipe(Box::new(log_file))) // 输出到文件
        .format(|buf, record| {
            // 自定义日志格式
            writeln!(
                buf,
                "{}|{}|{}|: {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                record.args()
            )
        })
        .init();

    Ok(())
}

//向量校验
pub fn validate_vector(s:&str)->Result<String>{
    let s=s.trim();
    let re = Regex::new(r"^\[(\s*[^\[\],\s]+(\s*,\s*[^\[\],\s]+)*)?\s*\]$").unwrap();
    if !re.is_match(s) {
        return Err(KvsError::StringError("Invalid vector format. Expected: [val1,val2,...]".to_string()));
    }

    let s=s.trim_matches(|c| c == '[' || c == ']');
    let split:Vec<&str>=s.split(',').collect();
    let len=split.len();
    if len==0{
        return Err(KvsError::StringError("Vector must have at least 1 dimension".to_string()));
    }

    let mut vecs=String::from("[");
    let mut i=0;
    for s in split{
        let res=s.trim().parse::<f32>();
        if res.is_err(){//非数字
            return Err(KvsError::StringError("Invalid input syntax for type vector".to_string()));
        }
        let num=res.unwrap();
        if num.is_nan() {
            return Err(KvsError::StringError("NAN not allowed in vector".to_string()));
        }
        if num.is_infinite(){
            return Err(KvsError::StringError("Inf not allowed in vector".to_string()));
        }
        vecs.push_str(s.trim());
        if i!=len-1{
            vecs.push_str(",");
        }
        i+=1;

    }
    vecs.push_str("]");
    Ok(vecs)
}