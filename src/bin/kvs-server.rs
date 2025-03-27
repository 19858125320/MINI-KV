use clap::Parser;
use kvs::{KvServer,Result,KvStore,SledStore,ThreadPool,ShardThreadPool,init_logger};
use log::{info, error, warn};
use std::env::current_dir;
use std::fs;
use std::net::SocketAddr;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::path::Path;


const DEFAULT_ADDRESS:&str="127.0.0.1:4001";

#[derive(Parser, Debug)]
#[command(name = "kvs-server", version, author, about = "A key value store server")]
struct KvsServer{
    /// The address to bind the server
    #[clap(short,long, default_value = DEFAULT_ADDRESS, value_parser = parse_addr)]
    addr: SocketAddr,

    /// The data directory to store the key-value pairs
    #[clap(short,long, default_value = "./data")]
    data: String,

    /// The log directory to store the log file
    #[clap(short,long, default_value = "./log")]
    log: String,

    /// The storage engine to use
    #[clap(short,long, default_value = "kvs")]
    engine: Option<Engine>,
}


#[derive(Clone,Copy,Debug,PartialEq,Eq)]
enum Engine {
    Kvs,
    Sled,
}

impl clap::ValueEnum for Engine {
    fn value_variants<'a>() -> &'a [Self] {
        &[Engine::Kvs, Engine::Sled]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        match self {
            Engine::Kvs => Some(clap::builder::PossibleValue::new("kvs")),
            Engine::Sled => Some(clap::builder::PossibleValue::new("sled")),
        }
    }
}

impl std::str::FromStr for Engine {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "kvs" => Ok(Engine::Kvs),
            "sled" => Ok(Engine::Sled),
            _ => Err(format!("Invalid engine '{}': must be 'kvs' or 'sled'", s)),
        }
    }
}

fn parse_addr(s:&str)->std::result::Result<SocketAddr,String>{
    s.parse::<SocketAddr>().map_err(|e|format!("Invalid address '{}': {}", s, e))
}

// fn init_logger(log_dir: &str) -> Result<()> {
//     // 确保日志目录存在
//     fs::create_dir_all(log_dir)?;

//     // 构造日志文件路径
//     let log_path = Path::new(log_dir).join("kvs.log");
//     let log_file = OpenOptions::new()
//     .write(true)
//     .append(true)
//     .create(true)
//     .open(log_path)?;

//     // 配置 env_logger
//     Builder::new()
//         .filter_level(log::LevelFilter::Info) // 设置日志级别
//         .target(env_logger::Target::Pipe(Box::new(log_file))) // 输出到文件
//         .format(|buf, record| {
//             // 自定义日志格式
//             writeln!(
//                 buf,
//                 "{} {} {} - {}",
//                 chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
//                 record.level(),
//                 record.target(),
//                 record.args()
//             )
//         })
//         .init();

//     Ok(())
// }

fn main(){
    //命令行参数解析
    let args=KvsServer::parse();
    //初始化日志
    init_logger(&args.log,false).unwrap();
    
    info!("Server starting with Address: {}",args.addr);
    
    //engine规则
    /*
        1.指定engine
            第一次启动server以指定的engine作为默认引擎
            非第一次启动得检查指定engine与上一次启动的engine是否一致，不一致报错
        2.未指定engine
            第一次启动server以kvs作为默认引擎
            非第一次启动得以上一次启动的engine作为默认引擎
     */

    let res=get_current_engine().and_then(move |cur_engine|{
        if let Some(engine)=args.engine{
            if let Some(cur_engine)=cur_engine{
                if engine!=cur_engine{
                    error!("Cannot specify engine '{:?}' because the current engine is '{:?}'",engine,cur_engine);
                    std::process::exit(1);
                }
            }
            Ok(engine)
        }else{
            Ok(cur_engine.unwrap_or(Engine::Kvs))
        }
    });

    if let Err(e)=res{
        error!("{}",e);
        std::process::exit(1);
    }
    let engine =res.unwrap();
    info!("Storage Engine:{:?}",engine);

    let path=current_dir().unwrap().join("engine");
    fs::write(path, format!("{:?}", engine)).unwrap();

    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();

    // 捕获 Ctrl+C 信号
    ctrlc::set_handler(move || {
        shutdown_clone.store(true, Ordering::SeqCst);
    }).expect("Error setting Ctrl+C handler");
    

    let pool=ShardThreadPool::new(4).unwrap();
    let data_path=args.data;
    if engine==Engine::Sled{
        let path=Path::new(&data_path).join("sled");
        let store=SledStore::open(path).unwrap();

        let mut server = KvServer::new(store, args.addr, shutdown,pool).unwrap();
        server.run().unwrap();
        server.shut_down();
    }else{
        let path=Path::new(&data_path).join("kvs");
        let store=KvStore::open(path).unwrap();
        
        let mut server = KvServer::new(store, args.addr, shutdown,pool).unwrap();
        server.run().unwrap();
        server.shut_down();
    }
    
    info!("Server shut down gracefully");
}

fn get_current_engine()->Result<Option<Engine>>{
    let path=current_dir()?.join("engine");
    if !path.exists(){//第一次启动
        return Ok(None);
    }

    match fs::read_to_string(path)?.parse(){
        Ok(engine) => Ok(Some(engine)),
        Err(e) => {
            warn!("The content of engine file is invalid: {}", e);
            Ok(None)
        }
    }
}