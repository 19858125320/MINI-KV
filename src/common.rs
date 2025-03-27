//use std::fmt::Result;

use crate::{Result,KvsError};
//请求协议格式
/* 
  4     1     4              4
<len><cmd><keylen><key>[<valuelen><value>] //其中value部分只有set命令才有
*/

#[derive(Copy,Clone,Debug,PartialEq,Eq)]
pub enum Cmd{
    Get(u8),
    Set(u8),
    Remove(u8),
}

impl Cmd{
    fn from_u8(c:u8)->Self{
        if c==1{
            return Cmd::Get(1);
        }else if c==2{
            return Cmd::Set(2);
        }else{
            return Cmd::Remove(3);
        }
    }
}
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct WrapCmd {
    pub cmd:Cmd,
    pub key:String,
    pub value:String,
}

impl WrapCmd{
    pub fn new_extra(cmd:Cmd,key:String,val:String)->Self{
        Self{cmd:cmd,key:key,value:val}
    }

    pub fn encode(&self)->Vec<u8>{
        let mut res=Vec::new();
        let mut fres=Vec::new();
        let mut len:u32=1;
        match self.cmd{
            Cmd::Get(c)=>{
                res.push(c);
                len+=4;
                len+=self.key.len() as u32;
                res.extend(u32::to_be_bytes(self.key.len() as u32));
                res.extend_from_slice(self.key.as_bytes());
            },
            Cmd::Set(c)=>{
                res.push(c);
                len+=8;
                len+=self.key.len() as u32;
                len+=self.value.len() as u32;
                res.extend(u32::to_be_bytes(self.key.len() as u32));
                res.extend_from_slice(self.key.as_bytes());
                res.extend(u32::to_be_bytes(self.value.len() as u32));
                res.extend_from_slice(self.value.as_bytes());
            },
            Cmd::Remove(c)=>{
                res.push(c);
                len+=4;
                len+=self.key.len() as u32;
                res.extend(u32::to_be_bytes(self.key.len() as u32));
                res.extend_from_slice(self.key.as_bytes());
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
        };
        //如果有value,解析value
        if let Cmd::Set(_)=cmd{
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
成功：OK[value]//只有Get响应有value
失败：Error<message>
*/

pub fn parse_response(s:String)->Result<String>{
    if s.starts_with("OK"){
        if s.len()>2{
            return Ok(s[2..].to_string());
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

// #[test]
// fn test_decode_get(){
//     let cmd=WrapCmd::new_extra(Cmd::from_u8(1), "mongodb".to_owned(), "".to_string());
//     let encode_val=cmd.encode();

//     let decode_val=WrapCmd::decode(encode_val).unwrap();
//     println!("decode value is: {:?}",decode_val);
//     assert_eq!(decode_val,cmd);
// }

// #[test]
// fn test_decode_set(){
//     let cmd=WrapCmd::new_extra(Cmd::from_u8(2), "mongodb".to_owned(), "mysql".to_string());
//     let encode_val=cmd.encode();

//     let decode_val=WrapCmd::decode(encode_val).unwrap();
//     println!("decode value is: {:?}",decode_val);
//     assert_eq!(decode_val,cmd);
// }
