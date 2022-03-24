# cita_recover

## 编译

默认SM2版本

```shell
cd cita_recover
$ cargo install --path .
```

如果想要编译secp256k1-keccak版本，需要修改`types/Cargo.toml`文件中的`features`
```
default = ["secp256k1", "sha3hash"]
```

### docker compile

```shell
cd cita_recover
$ docker build -t cita/cita_recover:sm .
```

## 使用方法

```shell
$ cita_recover --help
cita-recover 
yubo
CITA Block Chain Node powered by Rust

USAGE:
    cita_recover [OPTIONS] [SUBCOMMAND]

OPTIONS:
    -d, --data <data_direction>    Set data dir [default: ./data]
    -h, --height <height>          Sets the destination height
        --help                     Print help information

SUBCOMMANDS:
    full_mode_recover    recover full mode node from state snapshot
    help                 Print this message or the help of the given subcommand(s)
```

### 从节点自身数据库恢复
* -d 指定CITA 的data目录地址
* -h 指定希望到达的高度h (消去比h 高度高的块)

#### example
```shell
$ cita_recover -d test-chain-0/data -h 100
```

#### 作用

该程序用于离线恢复 CITA 的链故障，需要修改 CITA 的chain，executor 的KV数据库，以及修改bft 的WAL 文件

#### 原理

1. 修改chain 的current hash 指向目的高度块的hash
2. 修改bft 的wal 的index 文件指向相应的高度，并且从chain 的db 中抽取proof 和 previous hash 到目标高度的wal 日志中
3. 修改executor的 current hash 指向目的块的hash

### 从快照恢复

```shell
$ cita_recover full_mode_recover --help
recover full mode node from state snapshot

USAGE:
    cita_recover full_mode_recover [OPTIONS]

OPTIONS:
    -b, --backup <backup_direction>    Sets the snapshot backup direction [default: ./backup]
    -d, --data <data_direction>        Set data dir [default: ./data]
    -h, --height <height>              Sets the destination height to recover
        --help                         Print help information
```

* -d 指定CITA 的data目录地址
* -h 指定希望到达的高度h (消去比h 高度高的块)
* -b 指定备份状态快照的数据目录，该目录必须有与块高h同名的快照数据目录

#### example
```shell
$ cita_recover full_mode_recover -d ./test-chain-0/data -h 100 --b ./backup
```

#### 原理

以example设置的数据为例
1. 修改chain 的current hash 指向目的高度块的hash
2. 修改bft 的wal 的index 文件指向相应的高度，并且从chain 的db 中抽取proof 和 previous hash 到目标高度的wal 日志中
3. 删除就有的statedb目录，将备份的状态./backup/10拷贝到./test-chain-0/data中并重命名为statedb

## 另外

1. 希望该程序永远不要用到
2. 从节点自身数据库可以理解为使节点块高数据以及状态数据回退；从快照恢复则是仅作块高数据回退而未做状态数据回退，状态数据statedb是直接替换的
3. 亲测可用