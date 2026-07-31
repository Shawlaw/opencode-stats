<h1 align="center">Shaw OpenCode Stats</h1>

<p align="center">
  <a href="https://ratatui.rs/"><img src="https://img.shields.io/badge/Built_With_Ratatui-000?logo=ratatui&logoColor=fff" alt="Built With Ratatui"></a>
</p>

<p align="center">
  <a href="./README.md">English</a> | 
  <a href="./README_CN.md">中文</a>
</p>

一个长期维护的 OpenCode Stats fork：面向 OpenCode 使用数据的终端统计面板。

<img src="images/overview.png" alt="screenshot" style="zoom:50%;" />

`shaw-oc-stats` 会读取 OpenCode 本地 SQLite 数据库或 JSON 导出文件，在终端中展示 token 使用量、成本估算、模型与提供商分布，以及最近 365 天的活跃热力图。它参考了 Claude Code `/stats` 的使用体验，但以本地可运行、可导出、可分享为目标做了独立实现。

> 如果你已经在使用 OpenCode，并希望快速看清自己的调用量、成本和活跃趋势，这个工具可以直接上手。
>
> 这是一个非官方的社区项目，与 OpenCode 官方团队没有隶属关系，也不代表官方立场。

## 功能特性

- 基于 `ratatui` 构建的终端仪表盘界面
- 自动读取 OpenCode 本地数据库，也支持通过 `--json` 加载导出文件
- 展示总 token、成本、会话数、消息数、提示数等概览指标
- 按模型、按提供商查看使用占比与明细
- 提供全部时间、最近 7 天、最近 30 天三种统计范围
- 内置最近 365 天活跃热力图，便于观察长期使用趋势
- 支持亮色 / 暗色主题启动参数
- 支持非交互式 `snapshot` 输出，可生成终端字符画或 PNG 图片，并按时间范围、每日或模型维度筛选
- 每日 snapshot 图表和模型 / 提供商图表显示精确 token 数值
- 支持将当前页面复制到剪贴板：优先导出图片分享卡片，失败时自动降级为文本摘要
- 支持本地缓存模型价格数据，并提供缓存更新 / 清理命令
- 输出数据统计方式和 `opencode stats` 和 `opencode stats --models` 的输出对齐，保持数据一致

## 预览

Shaw OpenCode Stats 提供了三种数据预览视角：

| 年视图                                                        | 模型使用                                                    | 提供商使用                                                        |
| ------------------------------------------------------------- | ----------------------------------------------------------- | ----------------------------------------------------------------- |
| <img src="images/overview.png" alt="screenshot" width="300"/> | <img src="images/models.png" alt="model uses" width="300"/> | <img src="images/providers.png" alt="provider uses" width="300"/> |

同时，每个页面都支持以透明底的卡片风格直接渲染到剪贴板，比如：

<img src="images/card.png" alt="card" style="zoom:50%;" />

## 安装

### 从 GitHub Release 下载预编译二进制

在 Releases 页面下载对应平台的压缩包，解压后直接运行 `shaw-oc-stats`。

当前 Release 工作流会构建以下平台：

- Windows `x86_64-pc-windows-msvc`
- macOS `x86_64-apple-darwin`
- macOS `aarch64-apple-darwin`
- Linux `x86_64-unknown-linux-gnu`
- Linux `x86_64-unknown-linux-musl`

### 从源码构建

```bash
git clone https://github.com/Shawlaw/opencode-stats.git
cd opencode-stats
cargo build --release
```

生成的可执行文件位于：

```bash
target/release/shaw-oc-stats
```

或者直接使用 git 路径构建

```bash
cargo install --git https://github.com/Shawlaw/opencode-stats.git
```

## 使用方法

### 默认启动

默认情况下，程序会自动寻找 OpenCode 本地数据库并加载数据：

```bash
shaw-oc-stats
```

### 指定数据库路径

```bash
shaw-oc-stats --db /path/to/opencode.db
```

### 指定 JSON 导出文件

```bash
shaw-oc-stats --json /path/to/export.json
```

### 指定主题

```bash
shaw-oc-stats --theme auto
shaw-oc-stats --theme dark
shaw-oc-stats --theme light
```

### 忽略占位用的零成本

默认情况下，`shaw-oc-stats` 会保持数据库里记录的成本值，包括 `cost: 0`，以兼容现有行为。如果你的 OpenCode 环境会把仍然有 token 消耗的响应写成 `cost: 0` 占位值，可以使用 `--ignore-zero` 将这些零值视为缺失成本并改为估算。

```bash
shaw-oc-stats --ignore-zero
```

### 直接输出 Snapshot

`snapshot` 子命令不会启动交互式界面。默认使用 `terminal` 格式，立刻将完整快照以字符画输出到标准输出，便于重定向、保存或在脚本中使用。快照包含概览、逐日精确 token 图表、模型统计和提供商统计。

```bash
shaw-oc-stats snapshot
```

指定时间范围：

```bash
shaw-oc-stats snapshot --range 7d
shaw-oc-stats snapshot --range 30d
shaw-oc-stats snapshot --range all
```

只输出每日图表或模型维度数据：

```bash
shaw-oc-stats snapshot --range 7d --daily
shaw-oc-stats snapshot --range 7d --model
```

保存为 PNG 图片时，使用 `--format image --output`；图片会渲染与终端相同的字符画内容和精确数值。输出路径必须以 `.png` 结尾。

```bash
shaw-oc-stats snapshot --range 7d --daily --format image --output usage-7d.png
shaw-oc-stats snapshot --range all --format image --output usage.png --theme dark
```

`--format terminal`（也接受 `ascii`）可显式指定终端字符画，`--all` 可显式请求完整快照（也是默认值）。每日图表的每一行都会输出日期、精确 token 数和按相对大小绘制的柱形；模型和提供商的 token 列同样不做 K/M 缩写。输入参数也可直接放在子命令后面：

```bash
shaw-oc-stats snapshot --json /path/to/export.json --range 7d --daily
```

### 缓存管理命令

查看本地价格缓存路径：

```bash
shaw-oc-stats cache path
```

更新本地价格缓存：

```bash
shaw-oc-stats cache update
```

清理本地价格缓存：

```bash
shaw-oc-stats cache clean
```

## 交互说明

程序启动后可通过键盘快速切换页面和统计范围：

- `Tab` / `Left` / `Right` / `h` / `l`：切换页面
- `Up` / `Down` / `j` / `k`：在 `Models` / `Providers` 页面中移动焦点
- `r`：循环切换统计范围
- `1` / `2` / `3`：快速切换时间范围
- `Ctrl+S`：复制当前页面到剪贴板
- `q` / `Esc` / `Ctrl+C`：退出程序

页面包含：

- `Overview`：总体使用概览
- `Models`：模型维度统计
- `Providers`：提供商维度统计

时间范围包含：

- `All time`
- `Last 7 days`
- `Last 30 days`

## 数据来源与价格计算

### 数据输入

`shaw-oc-stats` 支持两种输入来源：

- OpenCode 本地 SQLite 数据库
- OpenCode 导出的 JSON 文件

默认数据库位置通常为：

- Windows: `%APPDATA%/opencode/opencode.db`
- Linux: `~/.local/share/opencode/opencode.db`
- macOS: `~/Library/Application Support/opencode/opencode.db`

### 价格数据

模型价格信息会优先从本地缓存读取，并在需要时从远程源刷新：

- 本地缓存路径：`~/.cache/shaw-oc-stats/models.json`
- 远程来源：`https://models.dev/api.json`
- 缓存有效期：1 小时

如果 OpenCode 配置中存在本地覆盖项，则会优先使用覆盖配置。

当完整价格信息不可用时，程序会使用回退策略估算缓存读写成本；如果数据库中已经记录了实际成本，也会优先采用数据库中的值。

如果数据库把非零 token 响应的 `cost: 0` 当作占位值，默认仍会保留该零值；传入 `--ignore-zero` 后才会改为估算成本。

## 适用场景

- 想快速查看自己在 OpenCode 中的 token 消耗情况
- 想按模型或提供商分析使用偏好
- 想了解最近一周、一个月或长期使用趋势
- 想将统计结果导出为图片或文本，便于分享
- 想在 CI、shell 脚本或重定向文件中获取精确的使用快照

## License

MIT

## 致谢

- 字体文件使用 [Cascadia Code](https://github.com/microsoft/cascadia-code)，遵循 SIL Open Font License
- 项目体验灵感参考了 Claude Code 的 `/stats`
- 参考项目：[ocmonitor-share](https://github.com/Shlomob/ocmonitor-share)
