# Poke v2 架构设计

> 日期: 2025-12-26
> 状态: 设计完成，待实现

## 1. 产品定位

**核心定位：** 本地优先的 EVM 生态开发与运维工具

**目标用户：**
- 智能合约开发者
- 节点运维人员
- 协议研究员

**设计哲学：** 借鉴 lazygit/k9s 的交互模式
- 对象 + 动作（选中什么就能操作什么）
- 面板并存（多个相关信息同屏显示）
- 命令系统（`:command` 快速导航和调用工具）
- 零配置即用

## 2. 整体架构

### 2.1 视图层级

```
L0: Dashboard 概览（默认视图）
    │
    ├─ [Enter/f] ──→ L1: Explorer 浏览器（全屏）
    │                    │
    │                    └─ [Enter] ──→ L2: 详情视图
    │                                       │
    │                                       └─ [t] ──→ L3: Trace 视图
    │
    └─ [:command] ──→ Toolkit 工具弹窗（覆盖层）
```

### 2.2 Dashboard 布局

```
┌─────────────────────────────────────────────────────────────┐
│ POKE - EVM Developer & Ops Toolkit          [?] Help       │
├─────────────┬───────────────────┬───────────────────────────┤
│ NODES       │ ACTIVITY          │ INSPECTOR                 │
│ ◉ anvil     │ Block #1234       │ (选中对象的详情)           │
│ ○ mainnet   │ > Tx 0xabc...     │                           │
│ ○ sepolia   │   Tx 0xdef...     │ Balance: 1.5 ETH          │
│             │ Block #1233       │ Nonce: 42                 │
├─────────────┤   ...             │ Code: Yes                 │
│ WATCHING    │                   │                           │
│ > Vault     │                   ├───────────────────────────┤
│   Router    │                   │ ACTIONS                   │
├─────────────┤                   │ p - poke  c - call        │
│ RECENT      │                   │ t - trace w - watch       │
│ > 0xabc...  │                   │                           │
└─────────────┴───────────────────┴───────────────────────────┘
: _                                              Status: OK
```

### 2.3 运维监控视图

```
┌─────────────────────────────────────────────────────────────┐
│ NODE: geth-mainnet                              [?] Help    │
├─────────────┬───────────────────────────────────────────────┤
│ HEALTH      │ SYNC PROGRESS                                 │
│ ● Overall   │ ████████████████░░░░ 82.3%                    │
│ ● Sync      │ Current: 19,234,567  Target: 23,400,000       │
│ ● Peers     │ Speed: 1,234 blocks/min  ETA: 2h 15m          │
│ ● RPC       │                                               │
├─────────────┼───────────────────────────────────────────────┤
│ METRICS     │ PEERS (12 connected)                          │
│ RPC Latency │ ▁▂▃▂▁▄▅▃▂▁  avg 45ms                         │
│ Block Time  │ ▃▃▂▄▅▃▂▃▄▂  avg 12.1s                        │
│ Gas Price   │ ▂▃▅▇▅▄▃▂▂▃  avg 25 gwei                      │
│ Pending Txs │ ▁▁▂▃▂▁▁▂▃▄  avg 4,521                        │
├─────────────┼───────────────────────────────────────────────┤
│ ALERTS (2)  │ RECENT LOGS                                   │
│ ⚠ Peer drop │ 12:34:05 Imported block #19234567            │
│ ⚠ High gas  │ 12:34:03 Peer connected 192.168.1.5          │
└─────────────┴───────────────────────────────────────────────┘
```

## 3. 命令系统

### 3.1 Toolkit 工具命令

**数据处理：**
| 命令 | 功能 | 示例 |
|------|------|------|
| `:encode` | ABI 编码 calldata | `:encode transfer(address,uint256)` |
| `:decode <data>` | ABI 解码 | `:decode 0xa9059cbb...` |
| `:hash <data>` | keccak256 | `:hash "transfer(address,uint256)"` |
| `:hex <value>` | hex/dec/string 互转 | `:hex 255` → `0xff` |

**查询转换：**
| 命令 | 功能 | 示例 |
|------|------|------|
| `:selector <sig>` | 计算函数选择器 | `:selector "transfer(address,uint256)"` |
| `:4byte <sel>` | 反查选择器 | `:4byte 0xa9059cbb` |
| `:convert <value>` | 单位转换 | `:convert 1.5 ether` |
| `:timestamp <ts>` | 时间戳转换 | `:timestamp 1703548800` |

**合约交互：**
| 命令 | 功能 | 示例 |
|------|------|------|
| `:call` | 只读调用合约 | `:call 0xRouter.swap(...)` |
| `:gas` | 估算 gas | `:gas 0xRouter.swap(...)` |
| `:slot` | 计算存储槽位置 | `:slot mapping 0 0xaddr` |

**地址计算：**
| 命令 | 功能 |
|------|------|
| `:create` | 计算 CREATE 地址 |
| `:create2` | 计算 CREATE2 地址 |
| `:checksum` | 地址校验和格式化 |

### 3.2 运维命令

| 命令 | 功能 |
|------|------|
| `:health` | 综合健康检查 |
| `:peers` | Peer 详情列表 |
| `:logs` | 实时日志流 |
| `:mempool` | 交易池状态 |
| `:rpc-stats` | RPC 统计 |
| `:admin` | 节点管理 |

### 3.3 节点管理命令

| 命令 | 功能 |
|------|------|
| `:connect <url>` | 连接新节点 |
| `:anvil` | 启动本地 Anvil |
| `:anvil --fork mainnet` | Fork 模式启动 |
| `:anvil kill` | 停止 Anvil |
| `:impersonate <addr>` | Anvil 模拟账户 |
| `:mine [n]` | Anvil 挖矿 |
| `:snapshot` | Anvil 快照 |
| `:revert <id>` | Anvil 回滚 |

### 3.4 导航命令

| 命令 | 功能 |
|------|------|
| `:blocks` | 跳转区块列表 |
| `:txs` | 跳转交易列表 |
| `:address <addr>` | 跳转地址详情 |
| `:trace <hash>` | 跳转交易 trace |

## 4. 技术架构

### 4.1 目录结构

```
src/
├── core/
│   ├── app.rs              # 精简：只管模式切换、全局状态
│   ├── command.rs          # 命令解析器
│   └── context.rs          # 上下文传递（选中对象、剪贴板）
│
├── modules/
│   ├── dashboard/          # L0: 概览面板
│   │   ├── mod.rs
│   │   ├── nodes_panel.rs
│   │   ├── activity_panel.rs
│   │   ├── watching_panel.rs
│   │   └── inspector_panel.rs
│   │
│   ├── explorer/           # L1: 浏览器
│   │   ├── mod.rs
│   │   ├── blocks.rs
│   │   ├── transactions.rs
│   │   ├── addresses.rs
│   │   └── trace.rs
│   │
│   ├── ops/                # 运维监控
│   │   ├── mod.rs
│   │   ├── health.rs
│   │   ├── peers.rs
│   │   ├── logs.rs
│   │   └── alerts.rs
│   │
│   ├── toolkit/            # 工具集
│   │   ├── mod.rs
│   │   ├── encode.rs
│   │   ├── decode.rs
│   │   ├── convert.rs
│   │   ├── hash.rs
│   │   ├── selector.rs
│   │   ├── slot.rs
│   │   └── ...
│   │
│   └── workflow/           # 工作流
│       ├── mod.rs
│       ├── anvil.rs
│       ├── watch.rs
│       ├── call.rs
│       └── nodes.rs
│
├── infrastructure/         # 保持现有结构
│   ├── abi/
│   ├── ethereum/
│   └── runtime/
│
├── ui/
│   ├── mod.rs
│   └── widgets/            # 可复用组件
│       ├── sparkline.rs
│       ├── panel.rs
│       └── ...
│
└── store/                  # 保持现有结构
```

### 4.2 模块接口

```rust
trait Module {
    /// 模块名称
    fn name(&self) -> &str;

    /// 处理按键事件
    fn handle_key(&mut self, key: KeyEvent, ctx: &mut Context) -> Action;

    /// 处理命令
    fn handle_command(&mut self, cmd: &Command, ctx: &mut Context) -> Action;

    /// 渲染模块
    fn render(&self, frame: &mut Frame, area: Rect, ctx: &Context);

    /// 模块是否需要更新（用于异步数据）
    fn needs_update(&self) -> bool { false }

    /// 更新模块状态
    fn update(&mut self, ctx: &mut Context) {}
}

enum Action {
    None,
    Navigate(Target),      // 导航到其他视图
    OpenToolkit(Command),  // 打开工具弹窗
    Copy(String),          // 复制到剪贴板
    Notify(String),        // 状态栏通知
}
```

### 4.3 上下文传递

```rust
struct Context {
    // 当前选中的对象
    selected: Option<Selected>,

    // 剪贴板
    clipboard: Option<String>,

    // 节点连接
    nodes: NodeManager,

    // 配置
    config: Config,

    // 标签
    labels: LabelStore,
}

enum Selected {
    Block(BlockNumber),
    Transaction(TxHash),
    Address(Address),
    TraceCall { tx: TxHash, path: Vec<usize> },
}
```

## 5. 实现优先级

### Phase 0: 核心重构
- [ ] 模块化架构重构
- [ ] 命令系统实现
- [ ] Dashboard 面板化布局
- [ ] Context 上下文系统

### Phase 1: 开发者工具
- [ ] `:encode` / `:decode`
- [ ] `:convert` / `:hex` / `:timestamp`
- [ ] `:selector` / `:4byte`
- [ ] `:hash`
- [ ] `:call` / `:gas`
- [ ] `:slot`
- [ ] `:create` / `:create2`

### Phase 2: 运维监控
- [ ] `:peers` 详情视图
- [ ] `:logs` 实时日志流
- [ ] 性能 sparklines
- [ ] `:health` 健康检查
- [ ] 告警系统
- [ ] `:mempool` 交易池

### Phase 3: 工作流增强
- [ ] Anvil 集成管理
- [ ] 多节点管理 UI
- [ ] Watch 监控增强
- [ ] 节点快照/回滚

## 6. 与现有代码的关系

| 现有功能 | 处理方式 | 目标位置 |
|----------|----------|----------|
| Block/Tx 列表 | 保留，迁移 | `modules/explorer/` |
| Trace 视图 | 保留，迁移 | `modules/explorer/trace.rs` |
| Poke 余额查询 | 保留 | `modules/workflow/poke.rs` |
| Storage 查询 | 保留 | `modules/explorer/addresses.rs` |
| ABI 解码 | 重构为命令 | `modules/toolkit/decode.rs` |
| 设置页面 | 重构 | `modules/settings/` |
| Watch 功能 | 增强 | `modules/workflow/watch.rs` |

## 7. 告警配置示例

```toml
[alerts]
peer_count_low = { threshold = 3, message = "Peer count critically low" }
sync_stalled = { duration = "5m", message = "Sync appears stalled" }
rpc_latency_high = { threshold = "500ms", message = "RPC latency spike" }
gas_price_high = { threshold = "100gwei", message = "Gas price elevated" }
```

## 8. 键位总览

### 全局
| 键 | 功能 |
|----|------|
| `:` | 打开命令面板 |
| `/` | 搜索/过滤 |
| `?` | 帮助 |
| `Tab` | 切换面板焦点 |
| `1-5` | 跳转到指定面板 |
| `Esc` | 返回/关闭弹窗 |
| `q` | 退出 |

### 列表操作
| 键 | 功能 |
|----|------|
| `j/k` | 上下移动 |
| `gg/G` | 顶部/底部 |
| `Enter` | 进入详情 |
| `f` | 全屏展开 |
| `Space` | 暂停/继续刷新 |

### 对象操作
| 键 | 功能 |
|----|------|
| `p` | Poke（查余额） |
| `t` | Trace |
| `c` | Call |
| `w` | Watch |
| `n` | 设置标签 |
| `y` | 复制 |
