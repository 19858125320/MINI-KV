use clap::Parser;
use kvs::{KvClient,Cmd,WrapCmd,KvsError,Result,init_logger};
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
    // #[command(subcommand)]
    // subcommand: Commands,
}

const DEFAULT_ADDRESS:&str="127.0.0.1:4001";

fn parse_addr(s:&str)->std::result::Result<SocketAddr,String>{
    s.parse::<SocketAddr>().map_err(|e|format!("Invalid address '{}': {}", s, e))
}

async fn parse_cmd(cmd:&str)->Result<WrapCmd>{
    let mut iter=cmd.split_whitespace();
    let cmd=iter.next().ok_or(KvsError::InvalidCommand)?;
    let wrap_cmd=match cmd{
        "get"=>{
            let key=iter.next().ok_or(KvsError::InvalidCommand)?;
            WrapCmd::new_extra(Cmd::Get(1), key.to_string(), "".to_string())
        }
        "set"=>{
            let key=iter.next().ok_or(KvsError::InvalidCommand)?;
            let value=iter.next().ok_or(KvsError::InvalidCommand)?;
            WrapCmd::new_extra(Cmd::Set(2), key.to_string(), value.to_string())
        }
        "remove"=>{
            let key=iter.next().ok_or(KvsError::InvalidCommand)?;
            WrapCmd::new_extra(Cmd::Remove(3), key.to_string(), "".to_string())
        }
        _=>{
            return Err(KvsError::InvalidCommand);
        }
    };
    Ok(wrap_cmd)
}

async fn handle_request(client:&mut KvClient,cmd:&str)->Result<()>{
    let cmd=parse_cmd(cmd).await?;
    info!("parse command success");
    let res=client.send_request(cmd.clone()).await;
    match res {
        Ok(response) => {
            if cmd.cmd==Cmd::Get(1){
                println!("{}",response);
            }
            else{
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
                        if line.eq_ignore_ascii_case("exit") {
                            println!("exit success");
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