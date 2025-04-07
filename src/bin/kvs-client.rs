use clap::Parser;
use kvs::common::{GetCmd,SetCmd,RemoveCmd,ScanCmd,DelVector, GetVector, SetVector,PingCmd};
use kvs::{init_logger, validate_vector, Cmd, KvClient, KvsError, Result};
use std::net::SocketAddr;
use tokio::signal;
use std::io::{self,Write};
use log::{warn,info};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

#[derive(Parser, Debug)]
#[command(name = "kvs-client", version, author, about = "A key value store client")]
struct KvsClient{
    #[arg(short,long,default_value=DEFAULT_ADDRESS,value_parser=parse_addr)]
    addr:SocketAddr,

    /// The log directory to store the client log file
    #[arg(short,long, default_value = "./log")]
    log: String,
}

const DEFAULT_ADDRESS:&str="127.0.0.1:4001";

fn parse_addr(s:&str)->std::result::Result<SocketAddr,String>{
    s.parse::<SocketAddr>().map_err(|e|format!("Invalid address '{}': {}", s, e))
}

async fn parse_cmd(cmd:&str)->Result<Cmd>{
    //处理ping命令
    let mut iter=cmd.split_whitespace();
    let cmd=iter.next().ok_or(KvsError::InvalidCommand)?;
    if cmd.eq_ignore_ascii_case("ping"){
        let mut message=String::from("");
        let it=iter.next();
        if it.is_some(){
            message=it.unwrap().to_string();
        }
        if iter.next().is_some(){
            return Err(KvsError::InvalidCommand);
        }
        return Ok(Cmd::Ping(PingCmd { message}));
    }
    let parts:Vec<&str>=cmd.splitn(2, ' ').collect();
    if parts.len()<2{
        return Err(KvsError::InvalidCommand);
    }
    
    let cmd=parts[0];
    let remain=parts[1].trim();

    let wrap_cmd=match cmd{
        "get"=>{
            let mut iter=remain.split_whitespace();
            let key=iter.next().ok_or(KvsError::InvalidCommand)?;
            if iter.next().is_some(){
                return Err(KvsError::InvalidCommand);
            }
            Cmd::Get(GetCmd { key: key.to_string()})
        }
        "set"=>{
            let mut iter=remain.split_whitespace();
            let key=iter.next().ok_or(KvsError::InvalidCommand)?;
            let value=iter.next().ok_or(KvsError::InvalidCommand)?;
            //过期时间 EX 5代表5秒
            let ex=iter.next();
            if ex.is_some(){
                if ex.unwrap().eq_ignore_ascii_case("EX"){
                    let ex=iter.next().ok_or(KvsError::InvalidCommand)?;
                    if iter.next().is_some(){
                        return Err(KvsError::InvalidCommand);
                    }
                    let ex:u32=ex.parse().map_err(|_|KvsError::StringError("expire time invalid".to_string()))?;
                    return Ok(Cmd::Set(SetCmd { key: key.to_string(), value: value.to_string(), expire: ex}));
                }else{
                    return Err(KvsError::InvalidCommand);
                }
            }
            Cmd::Set(SetCmd { key: key.to_string(), value: value.to_string(), expire: 0 })
        }
        "remove"=>{
            let mut iter=remain.split_whitespace();
            let key=iter.next().ok_or(KvsError::InvalidCommand)?;
            if iter.next().is_some(){
                return Err(KvsError::InvalidCommand);
            }
            Cmd::Remove(RemoveCmd { key: key.to_string()})
        }
        "scan"=>{
            let mut iter=remain.split_whitespace();
            let start=iter.next().ok_or(KvsError::InvalidCommand)?;
            let end=iter.next().ok_or(KvsError::InvalidCommand)?;
            if iter.next().is_some(){
                return Err(KvsError::InvalidCommand);
            }
            Cmd::Scan(ScanCmd { start: start.to_string(), end: end.to_string()})
        }
        "vget"=>{
            let mut iter=remain.split_whitespace();
            let key=iter.next().ok_or(KvsError::InvalidCommand)?;
            if iter.next().is_some(){
                return Err(KvsError::InvalidCommand);
            }
            Cmd::VGet(GetVector { key: key.to_string()})
        }
        "vset"=>{
            let parts:Vec<&str>=remain.splitn(2, ' ').collect();
            if parts.len()!=2{
                return Err(KvsError::InvalidCommand);
            }
            let key=parts[0];
            let value=parts[1];
            let value=validate_vector(value)?;
            // if iter.next().is_some(){
            //     return Err(KvsError::InvalidCommand);
            // }
            //校验value是否符合vector格式
            Cmd::VSet(SetVector { key: key.to_string(), value: value, expire: 0 })
        }
        "vdel"=>{
            let mut iter=remain.split_whitespace();
            let key=iter.next().ok_or(KvsError::InvalidCommand)?;
            if iter.next().is_some(){
                return Err(KvsError::InvalidCommand);
            }
            Cmd::VDel(DelVector { key: key.to_string()})
        }
        _=>{
            return Err(KvsError::InvalidCommand);
        }
    };
    Ok(wrap_cmd)
}

async fn handle_request(client:&mut KvClient,cmd:&str)->Result<()>{
    let cmd=parse_cmd(cmd).await?;
    let res=client.send_request(cmd.clone()).await;
    match res {
        Ok(response) => {
            if let Cmd::Get(_)=cmd{
                println!("{}",response);
            } else if let Cmd::Scan(_)=cmd{
                let v:Vec<&str>=response.split_whitespace().collect();
                for s in v{
                    println!("{}",s);
                }
            }else if let Cmd::VGet(_)=cmd{
                println!("{}",response);
            }else if let Cmd::Ping(_)=cmd{
                println!("{}",response);
            }else{
                println!("Ok");
            }
        },
        Err(KvsError::KeyNotFound)=>{
            println!("Key not found");
        }
        Err(e) => {
            return Err(e);
        },
    }
    io::stdout().flush()?;
    Ok(())
}

fn print_welcome() -> Result<()> {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);

    // 更精细的 ASCII 艺术
    let ascii_art = r#"
    ██████   ██████ █████ ██████   █████ █████            █████   ████ █████   █████
    ░░██████ ██████ ░░███ ░░██████ ░░███ ░░███            ░░███   ███░ ░░███   ░░███ 
     ░███░█████░███  ░███  ░███░███ ░███  ░███             ░███  ███    ░███    ░███ 
     ░███░░███ ░███  ░███  ░███░░███░███  ░███  ██████████ ░███████     ░███    ░███ 
     ░███ ░░░  ░███  ░███  ░███ ░░██████  ░███ ░░░░░░░░░░  ░███░░███    ░░███   ███  
     ░███      ░███  ░███  ░███  ░░█████  ░███             ░███ ░░███    ░░░█████░   
     █████     █████ █████ █████  ░░█████ █████            █████ ░░████    ░░███     
    ░░░░░     ░░░░░ ░░░░░ ░░░░░    ░░░░░ ░░░░░            ░░░░░   ░░░░      ░░░
    "#;

    // 渐变颜色效果
    let lines: Vec<&str> = ascii_art.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if i < 6 { 
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))?;
        } else if i < 8 { 
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true))?;
        } else { 
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
        }
        writeln!(&mut stdout, "{}", line)?;
    }

    // 版本号和提示
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Magenta)).set_italic(true))?;
    writeln!(&mut stdout, "\nServer Version: v1.0")?;
    stdout.reset()?;
    writeln!(&mut stdout, "Type 'exit' to quit or 'help' for commands.")?;

    Ok(())
}

#[tokio::main]
async fn main()->Result<()>{
    let kvs = KvsClient::parse();
    //初始化日志
    init_logger(&kvs.log,true)?;
    
    let mut client=KvClient::new(kvs.addr).await?;

    print_welcome()?;
    loop {
        // 打印提示符
        print!("mini-kv> ");
        io::stdout().flush()?;
        tokio::select! {
            // 处理 Ctrl+C 信号
            _ = signal::ctrl_c() => {
                warn!("Received Ctrl+C, shutting down...");
                println!("exit success");
                break;
            }
            // 读取用户输入并发送请求
            result = tokio::task::spawn_blocking(|| {
                let mut line = String::new();
                io::stdin().read_line(&mut line).map(|_| line)
            }) => {
                match result {
                    Ok(Ok(line)) => {
                        let line = line.trim();
                        if line.is_empty() || line=="\n" || line=="\r\n" {
                            continue;
                        }
                        if line.eq_ignore_ascii_case("exit") {
                            println!("exit success");
                            info!("Received exit command, exit normal");
                            break;
                        }
                       
                        // 发送请求并打印响应
                        match handle_request(&mut client,line).await {
                            Ok(_) => {},
                            Err(e) => println!("{}", e),
                        }
                    }
                    Ok(Err(e)) => {
                        println!("Failed to read input, {}", e);
                        break;
                    }
                    Err(e)=>{
                        println!("Failed to read input, {}", e);
                        break;
                    }
                    
                }
            }
        }
    }
    
    Ok(())
}