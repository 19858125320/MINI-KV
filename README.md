<div align="center">
<!-- 语言切换栏（带背景和圆角） -->
<div style="margin: 20px auto; padding: 12px; 
            background: #f8f9fa; border-radius: 10px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.05);
            width: fit-content;">
  <strong>
    <a href="./README.md" style="margin: 0 10px; color: #2c3e50; text-decoration: none;font-size: 18px;">🇨🇳 中文</a>
    <span style="color: #ddd;">|</span>
    <a href="./README-EN.md" style="margin: 0 10px; color: #2c3e50; text-decoration: none;font-size: 18px;">🇺🇸 English</a>
  </strong>
</div>

</div>

```
    ██████   ██████ █████ ██████   █████ █████            █████   ████ █████   █████
    ░░██████ ██████ ░░███ ░░██████ ░░███ ░░███            ░░███   ███░ ░░███   ░░███ 
     ░███░█████░███  ░███  ░███░███ ░███  ░███             ░███  ███    ░███    ░███ 
     ░███░░███ ░███  ░███  ░███░░███░███  ░███  ██████████ ░███████     ░███    ░███ 
     ░███ ░░░  ░███  ░███  ░███ ░░██████  ░███ ░░░░░░░░░░  ░███░░███    ░░███   ███  
     ░███      ░███  ░███  ░███  ░░█████  ░███             ░███ ░░███    ░░░█████░   
     █████     █████ █████ █████  ░░█████ █████            █████ ░░████    ░░███     
    ░░░░░     ░░░░░ ░░░░░ ░░░░░    ░░░░░ ░░░░░            ░░░░░   ░░░░      ░░░
```
# MINI-KV键值存储引擎
Rust实现的键值存储引擎，保证数据持久化到文件，防止丢失。(本项目用作Rust练习目的，不可用于生产环境!!!)

## 项目结构

```
MINI-KV
├── src
│   ├── bin
│   │   ├── kvs-client.rs               # 客户端程序入口
│   │   └── kvs-server.rs               # 服务端程序入口
│   ├── engines
│   │   ├── kvs.rs               # 本地存储引擎
│   │   └── sled.rs              # 第三方sled引擎
│   ├── thread_pool
│   │   ├── native.rs               # 虚假的线程池
│   │   └── shard.rs                # 基于channel的线程池
│   ├── client.rs                   # 客户端核心处理逻辑
│   ├── server.rs                   # 服务端核心处理逻辑
│   ├── common.rs                   # 公共模块，如数据的编解码，消息的解析等
│   └── error.rs                    # 错误定义
├── benches                             # 基准测试
├── tests                               # 测试用例
├── README.md                           # 中文版README
└── README-EN.md                        # 英文版README
```

## 源码编译
### 1、客户端编译
```sh
cargo build --bin kvs-client
```
### 2、服务端编译
```sh
cargo build --bin kvs-server
```  

## 服务端
### 1 简介 
服务端功能是接收客户端发来的get/set/remove请求并处理，将处理后的响应发给客户端，支持优雅关闭。

### 2 命令行使用 
```
 kvs-server --help: 查看使用说明 
```
```
 kvs-server [-a/--addr] [-e/--engine] [-d/--data] [-l/--log]
``` 
- --addr: 指定启动的ip和监听端口，默认为：**127.0.0.1：4001**  
- --engine: 指定存储引擎，默认为kvs.目前总共有[sled,kvs]两种引擎
- --data:指定数据存储目录，默认为: ./data下
- --log: 指定日志写入路径，默认为: ./log下

## 客户端
### 1 简介

客户端是一个交互式shell，主要发送get/set/remove命令给服务端并获取响应 

### 2 命令行使用
```
kvs-client --help: 查看使用说明 
```
```
kvs-client [-a/--addr] [-l/--log]
```
- --addr: 可选参数，用来指定服务端的ip,port,默认为：**127.0.0.1:4001**  
- --log: 可选参数，指定客户端日志输出目录，默认为: ./log

### 支持的功能
- **set key value:** 插入
- **get key:** 查询
- **scan start end:** 返回所有满足start <= key <= end的数据
- **remove key:** 删除key

## 待完成功能
- 服务端使用tokio重构
- 添加raft支持多副本
- 抽象出来一个解析模块
- 扩展通信协议实现更丰富的功能
- 实现基于LSM的存储引擎
- 支持mvcc
- 支持事务
## 贡献

欢迎提交问题和拉取请求。