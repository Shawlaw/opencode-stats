<h1 align="center">Shaw OpenCode Stats</h1>

<p align="center">
  <a href="https://ratatui.rs/"><img src="https://img.shields.io/badge/Built_With_Ratatui-000?logo=ratatui&logoColor=fff" alt="Built With Ratatui"></a>
</p>

<p align="center">
  <a href="./README.md">English</a> | 
  <a href="./README_CN.md">中文</a>
</p>

A maintained fork of OpenCode Stats: a terminal dashboard for OpenCode usage statistics.

<img src="images/overview.png" alt="screenshot" style="zoom:50%;" />

`shaw-oc-stats` reads your local OpenCode SQLite database or JSON export files and displays token usage, cost estimates, model and provider distributions, and a 365-day activity heatmap directly in your terminal. Inspired by Claude Code's `/stats` command, this is an independent implementation focused on local execution, export, and shareability.

> If you're already using OpenCode and want a quick view of your usage, costs, and activity trends, this tool is ready to go.
>
> This is an unofficial community project and is not affiliated with, endorsed by, or maintained by the OpenCode team.

## Features

- Terminal dashboard UI built on `ratatui`
- Automatically reads OpenCode local database, or loads export files via `--json`
- Displays total tokens, costs, session count, message count, prompt count, and other overview metrics
- View usage breakdown by model and provider
- Supports three time ranges: all time, last 7 days, and last 30 days
- Built-in 365-day activity heatmap for observing long-term usage trends
- Supports dark / light theme via command-line argument
- Non-interactive `snapshot` output as terminal ASCII art or PNG, with time-range, daily, and model filters
- Exact token values in daily snapshots and model/provider charts
- Copy current view to clipboard: prioritize image export (share card), with automatic fallback to text summary
- Local model pricing cache with update / clean commands
- Output data calculation aligns with `opencode stats` and `opencode stats --models` for consistency

## Preview

Shaw OpenCode Stats provides three data perspectives:

| Year view                                                     | Model usage                                                 | Provider usage                                                    |
| ------------------------------------------------------------- | ----------------------------------------------------------- | ----------------------------------------------------------------- |
| <img src="images/overview.png" alt="screenshot" width="300"/> | <img src="images/models.png" alt="model uses" width="300"/> | <img src="images/providers.png" alt="provider uses" width="300"/> |

Each page also supports exporting a transparent-background share card directly to clipboard:

<img src="images/card.png" alt="card" style="zoom:50%;" />

## Installation

### Download pre-built binaries from GitHub Releases

Download the archive for your platform from the Releases page, extract it, and run `shaw-oc-stats` directly.

The current release workflow builds for:

- Windows `x86_64-pc-windows-msvc`
- macOS `x86_64-apple-darwin`
- macOS `aarch64-apple-darwin`
- Linux `x86_64-unknown-linux-gnu`
- Linux `x86_64-unknown-linux-musl`

### Build from source

```bash
git clone https://github.com/Shawlaw/opencode-stats.git
cd opencode-stats
cargo build --release
```

The compiled binary will be located at:

```bash
target/release/shaw-oc-stats
```

Or build directly using the git path:

```bash
cargo install --git https://github.com/Shawlaw/opencode-stats.git
```

## Usage

### Default launch

By default, the program automatically locates your OpenCode local database and loads the data:

```bash
shaw-oc-stats
```

### Specify database path

```bash
shaw-oc-stats --db /path/to/opencode.db
```

### Specify JSON export file

```bash
shaw-oc-stats --json /path/to/export.json
```

### Specify theme

```bash
shaw-oc-stats --theme auto
shaw-oc-stats --theme dark
shaw-oc-stats --theme light
```

### Ignore placeholder zero-cost values

By default, `shaw-oc-stats` keeps stored costs as-is, including `cost: 0`, to preserve compatibility with existing behavior. If your OpenCode setup stores `cost: 0` as a placeholder for responses that still have token usage, use `--ignore-zero` to treat those zero values as missing and estimate the cost instead.

```bash
shaw-oc-stats --ignore-zero
```

### Print a snapshot directly

The `snapshot` subcommand skips the interactive UI. Its default `terminal` format immediately writes a complete ASCII-art snapshot to standard output, making it suitable for redirection and scripts. The snapshot includes an overview, exact per-day token chart, model usage, and provider usage.

```bash
shaw-oc-stats snapshot
```

Choose a time range:

```bash
shaw-oc-stats snapshot --range 7d
shaw-oc-stats snapshot --range 30d
shaw-oc-stats snapshot --range all
```

Print only the daily chart or model breakdown:

```bash
shaw-oc-stats snapshot --range 7d --daily
shaw-oc-stats snapshot --range 7d --model
```

To save a PNG image, use `--format image --output`. The image renders the same ASCII-art content and exact values as the terminal output. The output path must end in `.png`.

```bash
shaw-oc-stats snapshot --range 7d --daily --format image --output usage-7d.png
shaw-oc-stats snapshot --range all --format image --output usage.png --theme dark
```

`--format terminal` (also accepted as `ascii`) explicitly selects terminal ASCII art, while `--all` explicitly requests the complete snapshot (and is the default). Every daily-chart row includes its date, exact token count, and a proportional bar; model and provider token totals are likewise not abbreviated to K/M. Input options can appear after the subcommand too:

```bash
shaw-oc-stats snapshot --json /path/to/export.json --range 7d --daily
```

### Cache management commands

View the local pricing cache path:

```bash
shaw-oc-stats cache path
```

Update the local pricing cache:

```bash
shaw-oc-stats cache update
```

Clean the local pricing cache:

```bash
shaw-oc-stats cache clean
```

## Interaction

Once the program is running, you can quickly navigate pages and time ranges using the keyboard:

- `Tab` / `Left` / `Right` / `h` / `l` — Switch pages
- `Up` / `Down` / `j` / `k` — Move focus within `Models` / `Providers` pages
- `r` — Cycle through time ranges
- `1` / `2` / `3` — Quickly switch time ranges
- `Ctrl+S` — Copy current view to clipboard
- `q` / `Esc` / `Ctrl+C` — Exit the program

Pages:

- `Overview` — Overall usage summary
- `Models` — Usage statistics by model
- `Providers` — Usage statistics by provider

Time ranges:

- `All time`
- `Last 7 days`
- `Last 30 days`

## Data sources and pricing

### Data input

`shaw-oc-stats` supports two input sources:

- OpenCode local SQLite database
- OpenCode exported JSON files

Default database locations:

- Windows: `%APPDATA%/opencode/opencode.db`
- Linux: `~/.local/share/opencode/opencode.db`
- macOS: `~/Library/Application Support/opencode/opencode.db`

### Pricing data

Model pricing is read from local cache first and refreshed from remote when needed:

- Local cache path: `~/.cache/shaw-oc-stats/models.json`
- Remote source: `https://models.dev/api.json`
- Cache TTL: 1 hour

If local overrides exist in your OpenCode configuration, they take precedence.

When complete pricing information is unavailable, the program falls back to estimated cache read/write costs. If the database already contains actual costs, those values are prioritized.

If the database stores `cost: 0` for a nonzero-token response, `shaw-oc-stats` keeps that stored zero by default. Pass `--ignore-zero` to estimate the cost instead.

## Use cases

- Quickly view your OpenCode token consumption
- Analyze usage preferences by model or provider
- Understand recent and long-term usage trends
- Export statistics as images or text for easy sharing
- Retrieve exact usage snapshots from CI, shell scripts, or redirected output

## License

MIT

## Acknowledgments

- Font: [Cascadia Code](https://github.com/microsoft/cascadia-code), under SIL Open Font License
- Experience inspired by Claude Code's `/stats` command
- Reference project: [ocmonitor-share](https://github.com/Shlomob/ocmonitor-share)
