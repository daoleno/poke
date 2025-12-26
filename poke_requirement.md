产品需求文档 (PRD): Poke (v0.1 MVP)
1. 产品概述
产品名称：Poke

定位：轻量级、零配置、本地化的以太坊节点 TUI（终端用户界面）交互工具。

核心价值：消除开发者在“终端代码”与“浏览器区块浏览器”之间的频繁切换，通过“戳一下”节点，即刻获得深度的实时反馈。
补充价值：在无浏览器环境（服务器/WSL/容器）中，提供完整的区块/交易/合约调试视图。

体验原则（借鉴优秀 TUI：lazygit / k3s / k9s）
1) 任意核心动作 <= 2 次键盘操作（搜索 / 切换 / 进入详情 / 返回）。
2) 实时数据可暂停、可回溯（避免“信息洪水”）。
3) 可发现性强：所有快捷键可通过 ? 打开帮助。
4) 失败即反馈：错误显示在状态栏，且不阻塞 UI。

2. 用户痛点与使用场景
场景 A：智能合约本地开发 (Anvil)
痛点：运行 anvil 后，终端只是一串冰冷的日志。想看某个测试账号的余额变化或某个合约的存储状态，必须不停地敲 cast balance 或 cast storage-at，效率极低。

Poke 解决方案：自动连接 Anvil，实时列出默认账号，一键监控合约变量变化。

场景 B：服务器节点巡检 (Geth/Reth)
痛点：SSH 进入服务器，想看节点同步进度（Syncing）或 Peer 连接情况，通常要进入 geth attach 运行 JS 命令，不够直观。

Poke 解决方案：在服务器终端直接运行 poke，以可视化的仪表盘展示节点健康度、Gas 价格走势和同步进度条。

场景 C：复杂交易调试
痛点：交易失败了，但在终端里只能看到一个哈希。去 Etherscan 看太慢，且本地环境没有浏览器。

Poke 解决方案：输入交易哈希，Poke 立即通过 debug_traceTransaction 在终端里展示调用栈分级视图，并尝试用本地 ABI 进行解码。

场景 D：无浏览器环境排障（服务器 / WSL / 容器）
痛点：远程机器没有 GUI 或浏览器，无法查看区块浏览器或可视化调试信息，只能靠反复命令 + 日志猜测。

Poke 解决方案：在纯终端内提供“区块/交易/合约”详情与 Trace 视图，替代浏览器依赖。

3. 功能需求 (MVP 范围)
3.1 智能连接管理器 (The "Plug")
自动探测：默认尝试连接 localhost:8545。

节点识别：通过 web3_clientVersion 识别节点类型（Anvil, Geth, Reth），并解锁特定功能（如 Anvil 的 impersonate）。

多端支持：支持通过参数指定远程 RPC 或 IPC 路径。

连接状态：展示当前连接端点、节点类型、最新块高、RPC 延迟。

失败恢复：RPC 超时或断连时，自动重试 + 状态栏提示；不阻塞 UI。

3.2 实时监控面板 (Live Dashboard)
区块流：实时滚动展示最新区块、高度、Gas 消耗、交易数量。

Gas 计价器：展示 Base Fee 的实时波动。

同步状态：如果是全节点，展示同步百分比和 Peer 数量。

列表容量控制：仅保留最近 N 个区块（默认 50，可配置）。

暂停/恢复：支持暂停刷新（Space）与快速恢复（r）。

3.3 账户与合约“戳一下” (The "Poke" Action)
资产快照：输入地址，展示各币种余额（本地可配常用代币列表）。

存储浏览器：针对合约地址，支持查看特定 Slot 的 Hex 内容。

状态追踪：支持锁定一个地址，当该地址发生任何交易时，终端发出视觉提醒。

地址详情页：展示最近 N 笔交易、合约标签、余额变化趋势（轻量 ASCII sparkline）。

3.4 交易解码器 (Tx Inspector)
基础解析：展示 From/To/Value/Input Data。

本地 ABI 联动：扫描当前目录及子目录下的 out/ 或 artifacts/ 文件夹。如果匹配到 ABI，自动将 Input Data 解析为人类可读的函数名和参数。

失败降级：ABI 不匹配时，显示原始 selector + hex data，并提示可加入 ABI。

Trace 视图（MVP-lite）：调用栈分层展示，支持折叠/展开；显示 gas 消耗与 revert reason（如可用）。

过滤输入：支持 from:/to:/method:/value: 形式的快速过滤（在交易列表中）。

4. 交互设计 (TUI Layout)
Poke 采用经典的三栏式或上下分层设计，支持全键盘操作：

视图层级与导航：
- Dashboard（默认）-> Address / Block / Tx 详情页 -> Trace 视图。
- Enter：进入详情；Esc：返回上一层；Tab：切换焦点区域。

快捷键定义（全局）：

Tab：在监控面板、账户视图、搜索框之间切换。

/：激活全局搜索（输入 Address, TxHash, 或 Block Number）。

s：进入设置（配置 ABI 路径、RPC 列表）。

q：退出。

快捷键定义（建议补充）：
Enter：进入详情视图；Esc：返回上一级。
Space：暂停/恢复实时刷新。
?：打开帮助（快捷键一览）。
f：固定/取消固定当前选中行（Pin）。
r：刷新当前视图。

UI 审美：

使用 ANSI 256 色，主色调为“赛博青”和“警示橙”。

数据更新时采用局部渲染，确保极速响应。

状态栏：
- 当前端点、节点类型、最新块高、RPC 延迟。
- 错误/告警提示（超时、断连、不同步）。

5. 技术架构建议
开发语言：Rust (推荐，生态与 Alloy 高度兼容，性能极佳)。

核心库：

Ratatui：负责 TUI 渲染。

Alloy (alloy-rs)：负责高性能 RPC 通信。

Tokio：处理多异步任务（同时监听区块更新和用户输入）。

数据存储：Stateless 优先。仅使用本地 SQLite 存储用户自定义的标签（如：给地址 0x123 命名为 "Vault"）。

配置文件：~/.config/poke/config.toml（RPC 别名、ABI 路径、代币列表、UI 配色）。

6. 非功能性需求
启动速度：从运行命令到显示界面，时间必须小于 200ms。

低资源占用：内存占用不超过 30MB。

零依赖：编译为单个二进制文件，无需安装 Node.js 或 Python 环境。

性能上限：
- 列表类视图默认只渲染可视区域 + 缓冲区（virtual list）。
- 大批量交易时自动分页或聚合。

稳定性：
- 单次 RPC 超时不超过 3s，超时后降级提示。
- 持续错误不阻塞 UI，可手动重连（R）。

7. 路线图 (Roadmap)
Week 1: 实现 RPC 自动探测与基础 Dashboard。

Week 2: 实现交互式搜索与账户资产快照。

Week 3: 实现本地 ABI 自动扫描与交易解析逻辑。

Week 4: 优化 TUI 动画与多节点切换体验。

Week 5 (可选): Trace 视图增强（折叠/展开、gas/revert 标注、过滤）。

Week 6 (可选): 插件/自定义命令（如对选中地址运行自定义脚本）。

8. 现状快照（便于接手理解）
8.1 主干现状（已跑通）
- 主干已跑通：HTTP/IPC 连接 + 节点识别/轮询、Block/Tx 列表、真实 debug_traceTransaction Trace、本地 ABI 扫描 + 函数名/参数解码、p 资产快照（ETH + ERC20）、o storage slot、watch、设置弹窗、鼠标 + vim 操作。
- 关键入口：事件泵/后台线程在 src/main.rs:165，RPC worker 在 src/rpc/worker.rs:100，ABI 扫描/解码在 src/abi/registry.rs:32 + src/abi/decode.rs:14。

8.2 遗留任务（建议按优先级）
P0：PRD 核心缺口 / 不稳定点
- 多节点管理器（The Plug）：支持多个 RPC/IPC 列表、别名、UI 内切换、自动探测优先级、断线重连策略（目前只有单端点重试）。入口：src/rpc/worker.rs:100、src/main.rs:165。
- ABI 扫描可配置 + 可重载：目前只在启动时扫 CWD 的 out/ 或 artifacts/；需要支持配置扫描路径、手动重载、缓存/增量。相关：src/abi/registry.rs:32，并且 abi_reload_sender 目前是“预留未接线”的技术债：src/app.rs:414。
- ABI 解码覆盖率：当前 decode_calldata 只覆盖有限类型（无 tuple/struct 等复杂类型的可靠支持）；需要补齐并加单测。相关：src/abi/decode.rs:14。
- Token 快照性能/正确性：现在每次 p 会对 tokens 做串行 eth_call，且 token 列表不区分链；需要做 batch JSON-RPC、并发限流、chainId 维度配置、错误展示更清晰。相关：src/config/mod.rs:6、src/rpc/worker.rs:403、src/main.rs:165。
- Trace 兼容性：不同节点 callTracer 返回字段可能不一致（尤其 input/error 信息）；需要更强的兼容/降级（无 debug namespace 时的提示与替代方案）。相关：src/rpc/worker.rs:40、src/app.rs:1874、src/ui/mod.rs:1074。

P1：工具体验/可发现性
- Tx 列表增强：把 label 显示进 tx 的 from/to（现在 label 只在地址/合约列表里），并支持 label: 过滤 Tx/Trace（目前 filter 的 label 主要落在地址/合约）。相关：src/app.rs:105（FilterKey）、src/app.rs:1411（matches_tx）、src/ui/mod.rs:394（Settings）、src/ui/mod.rs:343（Help）。
- 设置页继续完善：当前 Settings 基本是只读 + r 重载 config（src/ui/mod.rs:394）；可加“重载 ABI/显示扫描耗时/显示当前 chainId/打开配置路径”等动作。
- 标签系统完善：已接 SQLite（src/store/labels.rs:8，启动加载在 src/main.rs:90），但还缺“标签管理视图/批量编辑/从 watch 列表显示 label”等。

P2：稳定性/工程化（接手者通常要做）
- RPC 成本优化：现在 block 拉取用 eth_getBlockByNumber(true) + receipt（部分）偏重；建议改成按需取 receipt/tx detail、或订阅 newHeads（如果后续加 WS）。相关：src/rpc/worker.rs:219 起的 block/receipt 逻辑。
- 启动与资源约束：ABI 扫描可能在大 repo 变慢；需要可关闭/限目录/缓存结果，避免违背 PRD 的 200ms 启动目标。相关：src/main.rs:236（ABI 扫描线程入口）。
- 测试/CI/发布：给 ABI 解码、slot 解析、filter 解析、token formatting 加单测；加 anvil/geth 最小集成测试；补 README/配置示例。

8.3 关键数据流（最小心智模型）
- 启动：main 解析 endpoint（--rpc/--ipc）→ spawn_worker（RPC 线程）→ spawn_abi_scanner（ABI 扫描线程）→ load config + labels DB → run_app（UI loop）。
- UI loop：每个 tick 先 pump_background（接收 RpcEvent/AbiRegistry，落地到 App state），再 draw UI；键盘/鼠标事件只改 App state（通过 pending_* 请求“发单”），真正的 RPC 在 worker 线程里跑。
- RPC worker：连接阶段识别 node_kind + 拉 head + 拉 snapshot；运行中每 500ms 轮询 head，发现新区块后补拉缺口区块；每 2s 拉 peerCount/syncing 并回推状态。

8.4 运行 / 配置 / 本地数据（接手 10 分钟上手）
- 运行：`cargo run -- --rpc http://localhost:8545` 或 `cargo run -- --ipc ~/.ethereum/geth.ipc`。
- 配置文件：优先 `POKE_CONFIG`；否则 `$XDG_CONFIG_HOME/poke/config.toml` 或 `~/.config/poke/config.toml`（见 src/config/mod.rs:43）。
- 数据目录：`$XDG_DATA_HOME/poke` 或 `~/.local/share/poke`；标签库默认 `labels.sqlite3`（见 src/config/mod.rs:58、src/main.rs:90）。
- ABI 扫描规则：启动时扫描 CWD 下所有 `*.json`，且路径中包含 `out` 或 `artifacts`；跳过常见大目录（.git/target/node_modules/...）以及 >5MB JSON（见 src/abi/registry.rs:36、src/main.rs:231）。

8.5 快捷键/命令速查（以实现为准）
- 全局：`?` 帮助；`s` 设置；`/` 搜索/过滤；`r` refresh；`Space` pause；`q` 退出。
- 导航：`Tab/h/l` 切焦点；`j/k` 上下；`gg/G` 顶/底；`Ctrl-u/d` 半页；`Ctrl-b/f` 翻页；`[` `]` 切 section；`1-5` 跳 section；`Enter` 进入；`Esc` 返回。
- 动作：`p` 查余额（ETH + tokens）；`o` 查 storage slot；`t` 打开 Trace；`e/Enter` 折叠/展开 Trace；`w` watch；`n` 设置 label。
- `/` 行为：单 token（块号/地址/txhash）会 jump；多 token 会按 filter 解析；`clear/reset/none` 清空 filter（见 src/app.rs:819、src/app.rs:2026）。

8.6 接手建议（建议先做“低风险高收益”）
- 先把 ABI reload 接线打通（哪怕只做“重新扫描 + UI 提示耗时/结果”），这是后续所有解码体验的地基。
- 再补 Tx/Trace 的 label 展示与 label: 过滤（提升可用性，改动集中在 App 过滤与 UI 渲染）。
- 最后再做多节点管理器与 token batch（牵涉 worker 生命周期/状态切换/并发与错误模型，范围更大）。
