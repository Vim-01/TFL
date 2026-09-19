# ⚡ TFL (Top For LLM)

> **Modern, High-Performance Host & Local LLM Inference Terminal Monitor**  
> Inspired by `btop`, purpose-built for AI engineers running local models (`llama.cpp`, `vLLM`, `Ollama`).

[![Rust](https://img.shields.io/badge/rust-1.98%2B-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Linux-green.svg)](https://kernel.org)

---

## 🌟 Overview

**TFL** is a blazingly fast, modern terminal user interface (TUI) tool written in **Safe Rust**. It bridges the gap between low-level hardware diagnostics (`btop`, `nvtop`) and high-level LLM runtime observability.

### Why TFL?
Traditional tools show GPU compute and total VRAM, but they are blind to the mechanics of modern transformer inference:
- They don't know the **difference between Model Weights and dynamic KV Cache**.
- They don't report **Prompt Prefill Speed vs Token Decode Speed**.
- They can't track **Time to First Token (TTFT)** or **Inter-Token Latency (ITL)**.
- They have no visibility into **Speculative Decoding / Multi-Token Prediction (MTP) draft acceptance rates**.
- They don't monitor **active slots or inference queue depths**.

**TFL delivers all of this in a unified, beautiful, responsive terminal dashboard.**

---

## 🖥️ Screenshots & Layout

```
⚡ TFL [Top For LLM] v0.1    [1:Dashboard]  [2:GPU]  [3:Slots]  [4:Help]     ● ONLINE │ 19:56:00
┌─ HARDWARE & VRAM ───────────────────────────────┐┌─ INFERENCE THROUGHPUT ─────────────────────────┐
│ GPU 0: AMD Radeon RX 7900 XTX (0000:07:00.0)    ││ TOKENS / SEC                                   │
│ PWR: 32W / 402W    TEMP: 32°C (Edge) 42°C (Hot) ││ 31.8 t/s (Decode)                              │
│ VRAM Usage: [████████████████████░░░] 24.0 GB   ││ Current Prefill:  1438 t/s                     │
│ ■ Used: 22.2G  ■ Free: 1.8G                     ││ Current Decode:   31.8 t/s                     │
│ Memory Bus / Controller Load: 9.0%              ││ TTFT: 120 ms      ITL: 31.4 ms                 │
│ ▅▅▅▆▇██▇▆▅▄▃▂▂▃▄▅▆▇                             ││ Decode Speed History (last 60s): Peak: 34.2 t/s│
│ GPU Compute Utilization: 12.0%                  ││   ▂▃▄▅▆▇██▇▆▅▄▃▂                               │
│ ▂▃▄▅▆▇██▇▆▅▄▃▂                                  ││                                                │
└─────────────────────────────────────────────────┘└────────────────────────────────────────────────┘
┌─ BACKEND & KV CACHE ────────────────────────────┐┌─ QUEUE & LATENCY ──────────────────────────────┐
│ Server Engine: llama.cpp   Model: Qwen3.8-27B   ││ Active Slots:      1 / 1 (BUSY)                │
│ Model Quant:   IQ3_S       Cache KV: K:q8_0 V:q8││ Pending Requests:  0   Evicted Blocks: 0       │
│ Speculative:   draft-dspark (Qwen3.8-DSpark-Q8) ││ Cache Hit Rate:    94.2%                       │
│ KV Cache Pool: [██████████░░░░░░░░░░░░] 22.4%   ││ Speculative / MTP: [████████████░░░] 68.5%     │
│ Context Window: [██████░░░░░░░░░░░░░░░] 29349/13││                                                │
└─────────────────────────────────────────────────┘└────────────────────────────────────────────────┘
┌─ CPU CORES LOAD (Histogram) ────────────────────┐┌─ ACTIVE SLOTS & TASKS ─────────────────────────┐
│ Total: 11%  |  Avg: 3.80 GHz  |  Cores: 12      ││ Slot   Task ID   Status     Prompt   Decoded   │
│ 100% ────────────────────────────────────────── ││ #0     71984     Generating 29349    420       │
│  75% ────────────────────────────────────────── ││                                                │
│  50% ────────────────────────────────────────── ││                                                │
│  25% ───█────────────────────────────────────── ││                                                │
│   0% ─█─█───█───█───█───█───█───█───█───█───█── ││                                                │
│       0 1   2   3   4   5   6   7   8   9  10 11││                                                │
└─────────────────────────────────────────────────┘└────────────────────────────────────────────────┘
 [q] Quit  [Tab] Next Tab  [p] Pause  [1-4] Direct View  [↑/↓] Select Item  [?] Help
```

---

## 🚀 Key Features

### 1. Universal Hardware Telemetry (AMD, NVIDIA, Intel & CPU)
- **Native AMD ROCm / Linux sysfs backend**: Direct reading from `/sys/class/drm/card*` and `/sys/class/hwmon`. Zero external C/FFI dependencies!
  - Edge, Hotspot (Junction), and VRAM Memory temperatures.
  - VRAM allocated vs Free, GTT Shared System Memory.
  - GPU compute load (%) and memory bus / controller load (%).
  - Real-time average wattage and TDP limit cap.
- **NVIDIA GPU Support**: Dynamic telemetry via `nvidia-smi` / NVML.
- **Heterogeneous Multi-GPU**: Supports systems with AMD iGPU + NVIDIA dGPU concurrently. Toggle between cards on the fly with `j`/`k` or `↑`/`↓`!

### 2. Multi-Core CPU Monitoring
- Real-time per-core load histogram with vertical unicode sub-block levels (` `, `▂`, `▃`, `▄`, `▅`, `▆`, `▇`, `█`).
- Color gradient based on load (Teal/Blue < 40%, Amber 40-75%, Coral/Red > 75%).
- Average clock frequency across cores (GHz) and global CPU load.
- RAM and Swap utilization.

### 3. Local LLM Runtime Inspector (`llama.cpp`, `vLLM`, `Ollama`)
- **Model Metadata**: Automatic discovery of model alias, quantization format (`IQ3_S`, `Q4_K_M`, `Q8_0`), parameter count.
- **Cache Quantization**: Tracks Key and Value quantization types (`-ctk q8_0`, `-ctv q8_0`).
- **Speculative Decoding / MTP**: Displays draft model name, draft quantization, and speculative engine (`draft-dspark`).
- **Throughput Metrics**: Separate tracking for **Prefill Speed (t/s)** and **Decode Speed (t/s)** with 60-second historical sparkline trends.
- **KV Cache Pool**: Real-time pool fill percentage and current context token usage vs maximum context window (`n_ctx`).
- **Active Slots**: Live table of parallel inference slots with prompt tokens processed, cache hits, decoded tokens, and task IDs.

---

## ⌨️ Keyboard Shortcuts

| Key | Action |
|:---:|:---|
| `1` | Switch to **Dashboard View** (Main 6-panel overview) |
| `2` | Switch to **Detailed GPU View** (Clocks, voltages, VRAM breakdown) |
| `3` | Switch to **Slots & Engine Inspector** (Individual inference tasks) |
| `4` or `?` | Switch to **Help & Metrics Guide** |
| `Tab` | Cycle forward to next tab |
| `Shift+Tab` | Cycle backward to previous tab |
| `p` | Toggle **Pause / Resume** live metric updates |
| `↑` / `k` | Select previous GPU or Slot |
| `↓` / `j` | Select next GPU or Slot |
| `q` or `Ctrl+C` | Quit TFL gracefully |

---

## 🛠️ Building & Installation

### Requirements
- Linux (Arch Linux, Ubuntu, Debian, Fedora, NixOS, etc.)
- Rust 1.80+ (`cargo`, `rustc`)

### Build from source
```bash
git clone https://github.com/your-org/tfl.git
cd tfl

# Build optimized release binary
cargo build --release

# Run
./target/release/tfl
```

### Install to system
```bash
cargo install --path .
```

---

## ⚙️ Configuration & CLI Options

```
Usage: tfl [OPTIONS]

Options:
  -u, --url <ENDPOINT_URL>      LLM inference backend endpoint URL [default: http://127.0.0.1:8080]
  -i, --interval <INTERVAL_MS>  Metrics polling interval in milliseconds [default: 1000]
      --log <LOG_PATH>          Path to debug log file [default: /tmp/tfl.log]
  -h, --help                    Print help
  -V, --version                 Print version
```

### Examples:
```bash
# Monitor local llama.cpp on port 8080 (default)
tfl

# Monitor remote or non-standard port with 500ms refresh
tfl -u http://192.168.1.100:8080 -i 500
```

---

## 🛡️ Architecture & Safety
- **100% Safe Rust**: Zero unsafe blocks in core application logic.
- **Resource Leak Free**: Background workers leverage RAII `Drop` cleanup to prevent orphaned tasks.
- **Zero-Allocation History**: Metrics history utilizes a fixed-capacity ring buffer (`HistoryRingBuffer<T, 60>`).
- **Clean Panic Recovery**: Installs a custom panic hook ensuring your terminal emulator is NEVER left in raw mode or alternate screen.

---

## 📄 License
Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
