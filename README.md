# cita_recover

## 使用

**-d 指定CITA 的data目录地址
**-h 指定希望到达的高度h (消去比h 高度高的块)

## 作用

该程序用于离线恢复 CITA 的链故障，需要修改 CITA 的chain，executor 的KV数据库，以及修改bft 的WAL 文件

## 原理

1. 修改chain 的current hash 指向目的高度块的hash
2. 修改bft 的wal 的index 文件指向相应的高度，并且从chain 的db 中抽取proof 和 previous hash 到目标高度的wal 日志中
3. 修改executor的 current hash 指向目的块的hash

## 另外

1. 希望该程序永远不要用到
2. 与快照不同之处是：快照是两头消，主要用于清除老数据，减少存储空间，本程序用于运维链的故障。
快照是在线修复，本程序是离线修复
3. 亲测可用
