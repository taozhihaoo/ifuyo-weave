# PERF — 性能实测数据

> 所有数字来自真实测量（charter #68：目标以真实测试为准，不允许编造）。
> 复现：`cargo run --release -p weave-files --example m1_perf [max_scale]`

## M1 File Core — 2026-10-03

- 环境：Windows 11 (10.0.26200)，x64，release 构建，用户 `%TEMP%`（NTFS）
- 方法：每规模 3 次取最快；fixture 为 `<dir>/fNNN.txt` 小文本（100 文件/目录）；
  哈希输入为 100 MiB 零值文件（上限吞吐，真实文件同量级受读取速度约束）

| 场景 | 结果 | 备注 |
| --- | --- | --- |
| scan 100 files | **2 ms**（101 entries） | ~50k entries/s |
| scan 1,000 files | **40 ms**（1,010 entries） | ~25k files/s |
| scan 10,000 files | **235 ms**（10,100 entries） | ~42k files/s；流式增量，无全量缓存 |
| cancel responsiveness | **11 ms** 至 Cancelled | 512 entries 处取消，安全点收尾 |
| hash 100 MiB | **66 ms**（~1.5 GiB/s） | 内存 O(256 KiB chunk)，非 O(file) |

对照 charter #68 目标：

- [x] 1 万文件扫描不阻塞 UI（235 ms 且运行于后台任务）
- [x] 取消操作可快速响应（11 ms）
- [x] 大文件流式处理（哈希常量内存）
- [x] 普通文件操作即时反馈（Inspector 为 stat + ≤8 KiB 嗅探）

1 GB+ 文件：未单独测量（同机制线性外推；M3 Duplicate Finder 引入真实大文件
语料时补测）。前端逐帧渲染开销未在本表（Electron/WebView 层，M11 Polish 复测）。

## M2 Rename / Organizer — 2026-10-04

- 环境同上（release 构建，%TEMP% NTFS）；fixture：1000 文件/目录平铺
- 方法：Plan 构建 / 执行 / Undo 分别计时；执行含事务记录

| 场景 | 结果 | 备注 |
| --- | --- | --- |
| plan 100 files | **5 ms** | |
| rename 100 + tx | **23 ms**（执行）/ undo **21 ms**（100/100 restored） | |
| plan 1,000 files | **34 ms** | |
| rename 1,000 + tx | **234 ms**（执行）/ undo **224 ms**（1000/1000 restored） | ~4.3k renames/s |
| plan 10,000 files | **509 ms** | |
| rename 10,000 + tx | **2,532 ms**（执行）/ undo **2,416 ms**（10000/10000 restored, 0 conflicts） | ~4k renames/s；逐条事务持久化在先 |
| rename 100 MiB file | **<1 ms** | rename 为元数据操作，O(1) 与内容无关 |

对照 charter #68：

- [x] 1 万文件批处理不阻塞 UI（后台任务；2.5s 全程可取消）
- [x] 取消在条目间安全点即时生效（M1 已测 11ms）
- [x] 内存：Plan 增量构建 + 事务逐条落盘，无全量内容缓存
- 10k Plan 的 IPC DTO 序列化未单独测量（前端虚拟化列表 M11 复测）
