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
# MINI-KV key-value storage engine
The key-value storage engine implemented in Rust ensures that data is persisted to files to prevent loss. (This project is used for Rust practice purposes and cannot be used in production environments!!!)

## Project Structure

```
MINI-KV
├── src
│   ├── bin
│   │   ├── kvs-client.rs               # Client program entry
│   │   └── kvs-server.rs               # Server program entry
│   ├── engines
│   │   ├── kvs.rs               # Local storage engine
│   │   └── sled.rs              # Third-party sled engines
│   ├── thread_pool
│   │   ├── native.rs               # Fake thread pool
│   │   └── shard.rs                # Channel-based thread pool
│   ├── client.rs                   # Client core processing logic
│   ├── server.rs                   # Server core processing logic
│   ├── common.rs                   # Common modules, such as data encoding and decoding, message parsing, etc.
│   └── error.rs                    # Error Definition
├── benches                             # Benchmarks
├── tests                               # Test Cases
├── README.md                           # Chinese version README
└── README-EN.md                        # English version README
```

## Build Source Code
### 1、Build Client
```sh
cargo build --bin kvs-client
```
### 2、Build Server
```sh
cargo build --bin kvs-server
```  

## Server
### 1 Introduction 
The server function is to receive and process get/set/remove requests from the client, send the processed response to the client, and support graceful shutdown.

### 2 Command Usage 
```
 kvs-server --help: View instructions 
```
```
 kvs-server [-a/--addr] [-e/--engine] [-d/--data] [-l/--log]
``` 
- --addr: Specify the startup IP and listening port, the default is：**127.0.0.1：4001**  
- --engine: Specify the storage engine. The default is kvs. Currently there are two engines: [sled, kvs]
- --data:Specify the data storage directory. The default is: ./data
- --log: Specify the log writing path, the default is: ./log

## Client
### 1 Introduction

The client is an interactive shell that mainly sends get/set/remove commands to the server and gets responses

### 2 Command Usage
```
kvs-client --help: View instructions
```
```
kvs-client [-a/--addr] [-l/--log]
```
- --addr: Optional parameter, used to specify the server's ip, port, default is：**127.0.0.1:4001**  
- --log: Optional parameter, specifies the client log output directory, default is: ./log

After entering the shell, enter set key value/get key/remove key to interact with the server to insert, search, and delete data.

## Contribution

Issues and pull requests are welcome.