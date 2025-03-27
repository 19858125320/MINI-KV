# kvs--键值存储引擎
Rust实现的键值存储引擎，保证数据持久化到文件，防止丢失。(本项目用作Rust练习目的，不可用于生产环境!!!)

## 源码编译
### 1、客户端编译
```sh
cargo build --bin kvs-client
```
### 2、服务端编译
```sh
cargo build --bin kvs-server
```  

## 服务端：
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
- --data:指定数据存储目录，默认为:./data下
- --log: 指定日志写入路径，默认为:./log下

## 客户端
### 1 简介

客户端主要发送get/set/remove命令给服务端并获取响应，使用cli命令行来与服务端交互  

### 2 命令行使用
```
kvs-client --help: 查看使用说明 
```
```
- kvs-client set key value [-a/--addr]: 插入key,value.  
```
- -a: 可选参数，用来指定服务端的ip,port,默认为：**127.0.0.1:4001**  
```
kvs-client get key [-a/--addr]:获取key对应的value  
```
```
kvs-client remove key [-a/--addr]:删除key对应的value
```

## 贡献

欢迎提交问题和拉取请求。