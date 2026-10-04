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

## M3 Duplicate Finder — 2026-10-03

- 环境同上（release 构建，%TEMP% NTFS）；复现：`cargo run --release -p weave-files --example m3_perf [scale]`
- 三级管线的诚实测量（M3 下 §103：不许只有形容词）：候选缩减率 =
  1 − full_hashed / candidate_files

### Scenario A — unique-size（size 唯一 ⇒ 不进候选）

| files | scan | candidates | partial | full | groups |
| --- | --- | --- | --- | --- | --- |
| 1,000 | 33 ms | 0 | 0 | 0 | 0 |
| 10,000 | 400 ms | 0 | 0 | 0 | 0 |
| 50,000 | 2,164 ms | 0 | 0 | 0 | 0 |

唯一 size 场景在 Filtering 阶段（最便宜的一层）即全部排除，
不触碰任何文件内容——50k 扫描 2.2s，全部为目录枚举 + lstat。

### Scenario B — 同 size 不同内容（10,000 × 64 KiB）

| files | scan | candidates | partial | full | reduction | groups |
| --- | --- | --- | --- | --- | --- | --- |
| 10,000 | 976 ms | 10,000 | 10,000 | 0 | **100%** | 0 |

全部进入候选；partial hash（头 4 KiB + 尾 4 KiB）区分出全部 10,000
个不同文件 ⇒ full hash 次数 0。partial 层把 64 KiB/文件的读取压到
8 KiB（12.5% I/O）。

### Scenario C — 大量 exact duplicates（10 种内容 × 1,000）

| files | scan | candidates | partial | full | groups | dup files | reclaimable |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 10,000 | 1,709 ms | 10,000 | 10,000 | 10,000 | 10（=10 种内容） | 10,000 | 81,838,080 B |

真实重复场景 full hash 无法省略（最终事实层，M3 §3）；分组数与
内容种数精确一致（断言在 example 内），GroupId 内容派生稳定。

### Scenario D — large files（32 × 32 MiB，一半重复）

| files | bytes | scan | full | groups | throughput |
| --- | --- | --- | --- | --- | --- |
| 32 | 1.00 GiB | 643 ms | 32 | 16 | **1.56 GiB/s** |

流式 SHA-256（256 KiB chunk）+ 头/尾定位读：内存 O(chunk + 候选元数据)，
与文件大小无关（M1 已断言 hash 内存常量；此处为整管线复核）。

对照 M3 目标：

- [x] 建立真实基线，证明算法没有明显浪费（A：0 内容读；B：partial 全裁剪；
  C：分组精确；D：1.56 GiB/s 流式）
- [x] 1 万文件扫描不阻塞 UI（后台任务，同 M1 管道）
- [x] 大文件流式 + 有界内存（Scenario D）
- 峰值内存：机制上界 = 256 KiB 哈希 chunk + 8 KiB partial 缓冲 + 候选元数据
  （每文件 ~200 B 量级）；未引入进程级 RSS 测量（与 M1 口径一致，M11 复测）

## M4 Text — 2026-10-04

- 环境同上（release 构建，%TEMP%/内存数据）；复现：`cargo run --release -p weave-text --example m4_perf`
- 内存模型（§137）：Transformer/Extractor/Formatter O(输入)；Compare 窗口
  表 ≈ 16 MiB 上界；输入受 TextLimits 分档限额（§104），超出结构化拒绝

### Transformer（§138：10k / 100k / 1M lines）

| 操作 | 10k lines | 100k lines |
| --- | --- | --- |
| trimLines | 1 ms | 11 ms |
| deduplicate | 1 ms | 17 ms |
| sort（稳定，忽略大小写） | 3 ms | 61 ms |
| find/replace（literal） | 0 ms | 2 ms |

1M lines：与 100k 同机制线性外推（scale 参数实测与 100k 重合——fixture
生成器在 ≥100k 时复用 100k 行数；真实 1M 列入 M7 批处理场景补测）。

### Extractor（URL）

| lines | elapsed | matches |
| --- | --- | --- |
| 10,000 | 0 ms | 2 |
| 100,000 | 1 ms | 2 |

Rust regex 线性时间（§140 design property：**引擎保证**，非"所有 regex 都
安全"的宣称）；结果受 max_extract_matches=10k 上限（§185）。

### Compare（§141）

| A / B | elapsed | hunks | degraded |
| --- | --- | --- | --- |
| 10k identical | 2 ms | 0 | false |
| 10k fully-different | **7 ms** | 1 | true |
| 100k identical | 35 ms | 0 | false |
| 100k fully-different | **102 ms** | 1 | true |

- identical 走前/后缀裁剪 O(n) 直通；fully-different 超出 2000² LCS 窗口
  ⇒ 诚实降级（单块替换 + degraded 标注，D42）
- **性能修复（性能证据驱动）**：Moved 配对原为 O(D×I) 双重循环 + 逐条
  values().any 消费检查——100k 全不同实测 169.6 s；HashMap 队列 + 消费
  HashSet 化后 **102 ms（1663×）**。该项属 §226 级风险（大 diff 冻结），
  已由分档限额 + 修复双重消除
- 峰值内存：窗口表 16 MiB 上界 + O(输入) 键向量；未做进程级 RSS 测量

### Formatter（JSON format，parser 驱动）

| input | elapsed |
| --- | --- |
| 1,016,671 B（30k 对象数组） | **17 ms** |

format 限额 1 MiB（TextLimits 最紧档）⇒ 同步预览最坏延迟有界（§106 决策：
限额使阻塞可证地 < 100 ms 量级；D44）。

对照 M4 目标：

- [x] 建立真实基线，证明算法没有明显浪费（§138–§141 全部有数字）
- [x] Regex 安全 = 引擎线性性质（§140 如实表述）
- [x] Compare 超限显式拒绝 + 降级标注（§141）

## M5 Data — 2026-10-04

- 环境同上（release 构建，内存 fixture）；复现：`cargo run --release -p weave-data --example m5_perf [rows]`
- 内存模型（§137）：解析/变换 O(输入)；Compare 窗口 16 MiB 上界；会话
  500k 行驻留上限（DataLimits，§88/§166 统一来源）

### CSV Scan / Filter / Sort（§158）

| rows | scan | filter | sort |
| --- | --- | --- | --- |
| 10,000 | 1 ms | 0 ms | 1 ms |
| 100,000 | 17 ms | 10 ms | 16 ms |

Filter = AND 非破坏视图（§20/§21）；Sort = 稳定 + null 恒最后（§22/§23）。

### JSONL Scan（§160）

| rows | elapsed | valid |
| --- | --- | --- |
| 10,000 | 4 ms | 10,000 |
| 100,000 | 48 ms | 100,000 |

### Conversion（§161）

| rows | csv→json | csv→jsonl |
| --- | --- | --- |
| 10,000 | 8 ms | 7 ms |
| 100,000 | 83 ms | 82 ms |

对照 M5 目标：

- [x] 建立真实基线（§158：每项有数字，无 "fast enough"）
- [x] Regex 安全 = 引擎线性性质（§140）
- [x] Compare 超窗降级 + P1 修复锚点 102 ms（M4 节）
- 峰值内存：会话 O(输入) + 视图索引向量；未做进程级 RSS 测量

## M6 Image — 2026-10-04

- 复现：weave-media 单元测试（含真实编解码 roundtrip）；专项 bench 列入
  M6（下）§157–§161
- 关键路径实测（测试内计时口径，release 构建）：
  - PNG decode 64×32 → inspect：<1 ms
  - PNG→JPEG 重编码（quality 95）：<5 ms
  - WebP lossless roundtrip（16×16）：<5 ms
- 内存模型：解码前 header 守卫（max_dimension 16384 / max_pixels 80M /
  decoded 256 MiB 预算），炸弹头直接拒绝（测试：12000×12000 与
  u32::MAX 边界）；预览有界 ≤2048 边长
- 解压炸弹/巨型维度守卫测试：limits::tests::dimension_guard_* 与
  decode_guard_via_inspect_on_crafted_header

## M7 Batch Engine（上）— 2026-10-04

- 复现：`cargo test -p weave-batch --release -- --ignored --nocapture`
  （engine_tests::perf_text_batch_1000 / perf_image_batch_100）
- 环境：Windows 11 (10.0.26200)，x64，release 构建，用户 `%TEMP%`（NTFS）

| 场景 | 结果 | 备注 |
| --- | --- | --- |
| text batch 1000 files（read+trim+write，逐条 Export） | **288.7 ms** | ~0.29 ms/file；每条独立 read/write，失败隔离天然成立 |
| image batch 100 files（512×384 PNG decode + Fit 1920×1080 防放大 + PNG re-encode） | **96.8 ms** | ~0.97 ms/file；含 ImageLimits header 守卫与 PNG 无损中转 |

对照 M7（上）目标：

- [x] 确定性输入枚举（§55 路径字典序；快照后 per-item 顺序执行）
- [x] 诚实进度（§30 per-item 计数经 JobTracker，无伪造百分比）
- [x] 千文件量级不阻塞 UI（execute 在 spawn_blocking；JobTracker 进度）
- [x] 取消语义（未开始 ⇒ Cancelled；阶段间安全点 §33/§34）
- 峰值内存：单 item 负载驻留（Bytes/Text/Image 三态），Job 级 O(1)
  额外；未做进程级 RSS 测量（大文件语料随（下）Journal/Resume 补测）

## M7 Batch Engine（下）— 2026-10-05

- 复现：`cargo test -p weave-batch --release -- --ignored --nocapture`
  （perf_text_batch_1000 / perf_image_batch_100 / perf_matrix_and_cancel_latency）
- 环境：Windows 11 (10.0.26200)，x64，release 构建，用户 `%TEMP%`（NTFS）

| 场景 | 结果 | 备注 |
| --- | --- | --- |
| text 100 files（read+trim+write，workers=1） | **29.6 ms** | ~0.30 ms/file |
| text 1,000 files | **277.4 ms** | 与（上）288.7 ms 同量级（快照序逐条） |
| text 10,000 files | **3.21 s** | ~0.32 ms/file——线性，无超线性退化（§180 资源审计口径） |
| workers=1 vs 4（2,000 files） | 581.0 ms → **363.4 ms**（1.6×） | 同计划同输入结果一致（§123 测试固化）；I/O 密集场景加速受磁盘串行化限制 |
| cancel latency（8,000 条 @1s 取消） | wall **1.0029 s**（成功 3,513 + 取消 4,487） | 取消请求→返回 ≈ 2.9 ms（安全点收尾，§132） |
| image 100 files（decode+fit+png） | 121.5 ms | 与（上）96.8 ms 同量级（含 on_item journal 追加开销） |

对照 M7（下）目标：

- [x] 10,000 items 线性吞吐，无内存持续增长（每条目隔离、Job 级 O(1)）
- [x] workers 1 vs 4 结果一致（§123/§124 确定性测试）且真实加速
- [x] 取消延迟毫秒级（§132），未开始条目终态 Cancelled、计数闭合
- 峰值内存：未做进程级 RSS 分档（§131）——payload 隔离 + workers 上限 8
  构成上界；列入 M11 Polish 复测（KNOWN LIMITATION，不宣称）

## M8 Documents（上）— 2026-10-05

- 复现：weave-documents / weave-batch 单元测试（synthetic fixtures 明确
  标识，§7）；大文档基线（§101/§102：100/500/1000 页 PDF、大型 Office）
  列 M8（下）专项——本节不宣称
- 实测（测试内计时，release）：
  - PDF inspect（4 页 synthetic）：<1 ms
  - Merge（3+2 页）+ 输出重解析校验：<2 ms
  - Split every 5（12 页 → 3 parts，各含重解析校验）：<5 ms
- 资源限额（§100 集中定义，保守初值）：file 512 MiB / archive 65,536
  entries / entry 256 MiB / decompressed total 1 GiB / xml part 128 MiB /
  pages 10,000 / sheets 1,024 / slides 2,048——待大文档实测校准

## M8 Documents（下）— 2026-10-05

- 复现：`cargo test -p weave-batch --release -- --ignored --nocapture`
  （document_stage_tests::perf_batch_document_inspection）+ fixture_tests
- 环境：Windows 11 (10.0.26200)，x64，release，NTFS；fixtures = synthetic
  （1 页 PDF / 最小 DOCX，gen_fixtures 产出 §132）

| 场景 | 结果 | 备注 |
| --- | --- | --- |
| batch document inspect 10 docs | **25.5 ms**（2.5 ms/doc） | 冷启动含前两次 IO 预热 |
| batch document inspect 100 docs | **158.9 ms**（1.58 ms/doc） | |
| batch document inspect 1,000 docs | **1.43 s**（1.43 ms/doc） | 线性，无超线性退化（§168 Throughput；§180 资源审计口径） |
| PDF 单操作（inspect/merge/split/rotate，synthetic 小文档） | 毫秒级 | 见 M8（上）节 |
- 大文档基线（§101/§102：500/1000 页、大型 Office）：pdf-large-200page
  fixture 已入 tests/fixtures（200 页 inspect 通过）；更大规模列 M9 复测
  （KNOWN LIMITATION 如实——本节不宣称）

## M9 Utilities（下）— 2026-10-05

- 复现：`cargo test -p weave-utilities`（file_digest tests）+ 批量吞吐
  `cargo test -p weave-batch --release -- --ignored --nocapture`
- 文件 checksum（流式表驱动增量）：100 KB 级 fixture 毫秒级；增量=整体
  golden 与 crc crate 交叉验证（§36）
- 批量 document inspect（M8 基准沿用）：10/100/1000 docs = 2.5/1.58/
  1.43 ms/doc 线性
- 大规模 hash 文件基线：M1 hash job 既有（100 MiB ≈ 66 ms，PERF M1 节）
  ——M9 复用不重测（§5 复用决策）

## M10 Workflow（下）— 2026-10-05

- 复现：`cargo test -p weave-workflow --release -- --ignored --nocapture`
  （e2e_tests::perf_workflow_matrix）；synthetic 1 页 PDF 输入 §132
- 环境：Windows 11 (10.0.26200)，x64，release

| steps | inputs | validate | plan | preview |
| --- | --- | --- | --- | --- |
| 1 | 1 | 10.4 µs | 53.7 µs | 247.9 µs |
| 1 | 100 | 0.8 µs | 1.58 ms | 3.67 ms |
| 1 | 1000 | 2.0 µs | 14.59 ms | 35.87 ms |
| 10 | 1000 | 4.3 µs | 13.93 ms | 139.58 ms |
| 30 | 1000 | 8.8 µs | 22.02 ms | 365.40 ms |

- 结论：validate 与 steps 数线性（µs 级）；plan/preview 与输入规模线性
  （1000 输入 ≤ 366ms，含快照 stat）
