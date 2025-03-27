# MINI-KV键值存储引擎
Rust实现的键值存储引擎，保证数据持久化到文件，防止丢失。(本项目用作Rust练习目的，不可用于生产环境!!!)

## 项目结构

```
MINI-KV
├── src
│   ├── bin
│   │   ├── kvs-client.rs               # 客户端程序入库
│   │   └── kvs-server.rs               # 服务端程序入库
│   ├── engines
│   │   ├── kvs.rs               # 本地kvs存储引擎
│   │   └── sled.rs              # 第三方sled引擎
│   ├── thread_pool
│   │   ├── native.rs               # 虚假的线程池
│   │   └── shard.rs                # 基于channel的线程池
│   ├── client.rs                   # 客户端核心处理逻辑
│   ├── server.rs                   # 服务端核心处理逻辑
│   ├── common.rs                   # 公有的一些实现，如数据的编解码
│   └── error.rs                    # 错误定义
├── benches                             # 基准测试
├── tests                               # 测试用例
└── README.md                           # 项目文档
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

进入shell后，输入set key value/get key/remove key与服务端交互，实现插入，查找，删除数据

## 贡献

欢迎提交问题和拉取请求。