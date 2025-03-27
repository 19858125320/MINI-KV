use clap::{Parser, Subcommand};
use kvs::{error::KvsError, error::Result};
use kvs::{client::KvClient,common::{Cmd,WrapCmd}};
use std::{process::exit,net::SocketAddr};

#[derive(Parser, Debug)]
#[command(name = "kvs-client", version, author, about = "A key value store client")]
struct KvsClient{
    #[command(subcommand)]
    subcommand: Commands,
}

const DEFAULT_ADDRESS:&str="127.0.0.1:4001";
#[derive(Subcommand, Debug)]
enum Commands{
    #[command(about = "Set the value of a string key to a string")]
    Set{
        #[arg(name = "KEY",help = "A string key", required = true)]
        key: String,
        #[arg(name = "VALUE",help = "The string value of the key", required = true)]
        value: String,
        #[arg(short,long,default_value=DEFAULT_ADDRESS,value_parser=parse_addr)]
        addr:SocketAddr,
    },
    #[command(about = "Get the string value of a given string key")]
    Get{
        #[arg(name = "KEY", help = "A string key", required = true)]
        key: String,
        #[arg(short,long,default_value=DEFAULT_ADDRESS,value_parser=parse_addr)]
        addr:SocketAddr,
    },
    #[command(about = "Remove a given key")]
    Rm{
        #[arg(name = "KEY",help = "A string key", required = true)]
        key: String,
        #[arg(short,long,default_value=DEFAULT_ADDRESS,value_parser=parse_addr)]
        addr:SocketAddr,
    },
}

fn parse_addr(s:&str)->std::result::Result<SocketAddr,String>{
    s.parse::<SocketAddr>().map_err(|e|format!("Invalid address '{}': {}", s, e))
}
fn main()->Result<()>{
    // 初始化日志
    //env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let kvs = KvsClient::parse();
    match kvs.subcommand{
        Commands::Get { key,addr }=>{
            let cmd=WrapCmd::new_extra(Cmd::Get(1), key, "".to_string());
            let mut client=KvClient::new(addr)?;
            let res=client.send_request(cmd);
            match res{
                Ok(v)=>{
                    println!("{}",v);
                },
                Err(KvsError::KeyNotFound)=>{
                    println!("Key not found");
                    exit(0);
                }
                Err(e)=>{
                    return Err(e);
                }
            }
        }
        Commands::Set { key,value,addr}=>{
           // info!("receive set command");
            let cmd=WrapCmd::new_extra(Cmd::Set(2), key, value);
            let mut client=KvClient::new(addr)?;
            let res=client.send_request(cmd);
            match res{
                Ok(_)=>{println!("Ok");},
                Err(e)=>{
                    return Err(e);
                }
            }
        }
        Commands::Rm { key,addr}=>{
            let cmd=WrapCmd::new_extra(Cmd::Remove(3), key, "".to_string());
            let mut client=KvClient::new(addr)?;
            let res=client.send_request(cmd);
            match res{
                Ok(_)=>{println!("Ok");},
                Err(KvsError::KeyNotFound)=>{
                    println!("Key not found");
                    exit(0);
                }
                Err(e)=>{
                    return Err(e);
                }
            }
        }
    };
    
    Ok(())
}