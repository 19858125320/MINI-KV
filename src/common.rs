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

#[derive(Clone,Debug,PartialEq,Eq)]
pub enum Cmd{
    Get(GetCmd),
    Set(SetCmd),
    Remove(RemoveCmd),
    Scan(ScanCmd),
    //以下是向量类型相关的命令
    VGet(GetVector),
    VSet(SetVector),
    VDel(DelVector),

    //ping
    Ping(PingCmd),
}

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct GetCmd{
    pub key:String,
}

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct SetCmd{
    pub key:String,
    pub value:String,
    pub expire:u32,
}

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct RemoveCmd{
    pub key:String,
}

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct ScanCmd{
    pub start:String,
    pub end:String,
}

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct GetVector{
    pub key:String,
}

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct SetVector{
    pub key:String,
    pub value:String,
    pub expire:u32,
}

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct DelVector{
    pub key:String,
}

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct PingCmd{
    pub message:String,
}

impl Cmd{
    pub fn to_string(&self)->String{
        match self{
            Cmd::Get(_)=>"Get".to_string(),
            Cmd::Set(_)=>"Set".to_string(),
            Cmd::Remove(_)=>"Remove".to_string(),
            Cmd::Scan(_)=>"Scan".to_string(),
            Cmd::VGet(_)=>"VGet".to_string(),
            Cmd::VSet(_)=>"VSet".to_string(),
            Cmd::VDel(_)=>"VDel".to_string(),
            Cmd::Ping(_)=>"Ping".to_string(),
        }
    }

    pub fn encode(&self)->Vec<u8>{
        let mut res=Vec::new();
        let mut fres=Vec::new();
        let mut len:u32=1;
        match self{
            Cmd::Get(c)=>{
                res.push(1 as u8);
                len+=4;
                len+=c.key.len() as u32;
                res.extend(u32::to_be_bytes(c.key.len() as u32));
                res.extend_from_slice(c.key.as_bytes());
            },
            Cmd::Set(c)=>{
                res.push(2 as u8);
                len+=12;
                len+=c.key.len() as u32;
                len+=c.value.len() as u32;
                res.extend(u32::to_be_bytes(c.key.len() as u32));
                res.extend_from_slice(c.key.as_bytes());
                res.extend(u32::to_be_bytes(c.value.len() as u32));
                res.extend_from_slice(c.value.as_bytes());
                res.extend(u32::to_be_bytes(c.expire));
            },
            Cmd::Remove(c)=>{
                res.push(3 as u8);
                len+=4;
                len+=c.key.len() as u32;
                res.extend(u32::to_be_bytes(c.key.len() as u32));
                res.extend_from_slice(c.key.as_bytes());
            },
            Cmd::Scan(c)=>{
                res.push(4 as u8);
                len+=8;
                len+=c.start.len() as u32;
                len+=c.end.len() as u32;
                res.extend(u32::to_be_bytes(c.start.len() as u32));
                res.extend_from_slice(c.start.as_bytes());
                res.extend(u32::to_be_bytes(c.end.len() as u32));
                res.extend_from_slice(c.end.as_bytes());
            },
            Cmd::VGet(c)=>{
                res.push(5 as u8);
                len+=4;
                len+=c.key.len() as u32;
                res.extend(u32::to_be_bytes(c.key.len() as u32));
                res.extend_from_slice(c.key.as_bytes());
            },
            Cmd::VSet(c)=>{
                res.push(6 as u8);
                len+=12;
                len+=c.key.len() as u32;
                len+=c.value.len() as u32;
                res.extend(u32::to_be_bytes(c.key.len() as u32));
                res.extend_from_slice(c.key.as_bytes());
                res.extend(u32::to_be_bytes(c.value.len() as u32));
                res.extend_from_slice(c.value.as_bytes());
                res.extend(u32::to_be_bytes(c.expire));
            },
            Cmd::VDel(c)=>{
                res.push(7 as u8);
                len+=4;
                len+=c.key.len() as u32;
                res.extend(u32::to_be_bytes(c.key.len() as u32));
                res.extend_from_slice(c.key.as_bytes());
            },
            Cmd::Ping(c)=>{
                res.push(8 as u8);
                len+=4;
                len+=c.message.len() as u32;
                res.extend(u32::to_be_bytes(c.message.len() as u32));
                res.extend_from_slice(c.message.as_bytes());
            },
        }
        fres.extend(u32::to_be_bytes(len));
        fres.extend_from_slice(res.as_slice());
        fres
    }

    pub fn decode(len:u32,s:Vec<u8>)->Result<Self>{
        if len> s.len() as u32{
            return Err(KvsError::DecodeError);
        }
        //解析cmd
        match s[0]{
            1=>{//get
                let bytes:[u8;4]=s[1..5].try_into().unwrap();
                let key_len=u32::from_be_bytes(bytes);
                let key=String::from_utf8(s[5..5+key_len as usize].to_vec()).unwrap();
                return Ok(Cmd::Get(GetCmd{key:key}));
            },
            2=>{//set
                let bytes:[u8;4]=s[1..5].try_into().unwrap();
                let key_len=u32::from_be_bytes(bytes);
                let key=String::from_utf8(s[5..5+key_len as usize].to_vec()).unwrap();

                let st=5+key_len as usize;
                let bytes:[u8;4]=s[st..st+4].try_into().unwrap();
                let val_len=u32::from_be_bytes(bytes);
                let val=String::from_utf8(s[st+4..st+4+val_len as usize].to_vec()).unwrap();

                let st=st+4+val_len as usize;
                let bytes:[u8;4]=s[st..st+4].try_into().unwrap();
                let ttl=u32::from_be_bytes(bytes);
                
                return Ok(Cmd::Set(SetCmd{key:key,value:val,expire:ttl}));
            },
            3=>{//remove
                let bytes:[u8;4]=s[1..5].try_into().unwrap();
                let key_len=u32::from_be_bytes(bytes);
                let key=String::from_utf8(s[5..5+key_len as usize].to_vec()).unwrap();
                return Ok(Cmd::Remove(RemoveCmd{key:key}));
            },
            4=>{//scan
                let bytes:[u8;4]=s[1..5].try_into().unwrap();
                let start_len=u32::from_be_bytes(bytes);
                let start=String::from_utf8(s[5..5+start_len as usize].to_vec()).unwrap();

                let st=5+start_len as usize;
                let bytes:[u8;4]=s[st..st+4].try_into().unwrap();
                let end_len=u32::from_be_bytes(bytes);
                let end=String::from_utf8(s[st+4..st+4+end_len as usize].to_vec()).unwrap();
                return Ok(Cmd::Scan(ScanCmd{start:start,end:end}));
            },
            5=>{
                let bytes:[u8;4]=s[1..5].try_into().unwrap();
                let key_len=u32::from_be_bytes(bytes);
                let key=String::from_utf8(s[5..5+key_len as usize].to_vec()).unwrap();
                return Ok(Cmd::VGet(GetVector{key:key}));
            }
            6=>{
                let bytes:[u8;4]=s[1..5].try_into().unwrap();
                let key_len=u32::from_be_bytes(bytes);
                let key=String::from_utf8(s[5..5+key_len as usize].to_vec()).unwrap();

                let st=5+key_len as usize;
                let bytes:[u8;4]=s[st..st+4].try_into().unwrap();
                let val_len=u32::from_be_bytes(bytes);
                let val=String::from_utf8(s[st+4..st+4+val_len as usize].to_vec()).unwrap();

                let st=st+4+val_len as usize;
                let bytes:[u8;4]=s[st..st+4].try_into().unwrap();
                let ttl=u32::from_be_bytes(bytes);
                
                return Ok(Cmd::VSet(SetVector{key:key,value:val,expire:ttl}));
            }
            7=>{
                let bytes:[u8;4]=s[1..5].try_into().unwrap();
                let key_len=u32::from_be_bytes(bytes);
                let key=String::from_utf8(s[5..5+key_len as usize].to_vec()).unwrap();
                return Ok(Cmd::VDel(DelVector{key:key}));
            }
            8=>{
                let bytes:[u8;4]=s[1..5].try_into().unwrap();
                let message_len=u32::from_be_bytes(bytes);
                let message=String::from_utf8(s[5..5+message_len as usize].to_vec()).unwrap();
                return Ok(Cmd::Ping(PingCmd{message:message}));
            }
            _=>{
                Err(KvsError::DecodeError)
            }
        }
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