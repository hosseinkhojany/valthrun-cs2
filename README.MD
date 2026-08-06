<div align="center">

  <h1>⚡ Valthrun Enhanced CS2</h1>

  <p>
    <b>An actively maintained, high-performance, ultra-smooth fork of Valthrun CS2.</b><br>
    Engineered for 144Hz, 240Hz, and 360Hz+ high-refresh-rate displays with zero ESP micro-stutter.
  </p>

  <p>
    <a href="https://github.com/iarhaaan/valthrun-enhanced/actions"><img src="https://img.shields.io/github/actions/workflow/status/iarhaaan/valthrun-enhanced/rust.yml?branch=master&style=for-the-badge&logo=github&label=Build%20Status" alt="Build Status"></a>
    <a href="https://valth.run/download"><img src="https://img.shields.io/badge/Valthrun%20Loader-Download-blue?style=for-the-badge&logo=windows" alt="Loader Download"></a>
    <img src="https://img.shields.io/badge/Language-Rust-orange?style=for-the-badge&logo=rust" alt="Rust">
    <img src="https://img.shields.io/badge/Anti--Cheat-100%25%20Read--Only-success?style=for-the-badge&logo=shield" alt="Read Only">
  </p>

</div>

---

## 🌟 Overview

**Valthrun Enhanced** is an open-source, read-only external gameplay overlay for Counter-Strike 2. While stock Valthrun provides a great foundation, default builds suffer from **64 Hz memory tick jitter**, **per-frame string search overhead**, and **Win32 lock contention**.

**Valthrun Enhanced** resolves these bottlenecks, delivering **silky-smooth 240FPS+ rendering** while remaining 100% read-only and non-injected.

---

## 🔥 Why Valthrun Enhanced is Better

| Feature / Metric | Stock Valthrun | Valthrun Enhanced ⚡ |
| :--- | :--- | :--- |
| **ESP Motion Smoothness** | 64 Hz memory tick jitter | **Silky-Smooth 240Hz+ Synchronized Pawn & Bone LERP** |
| **Entity Lookup Speed** | $O(\log N)$ `BTreeMap` search | **$O(1)$ High-Speed `HashMap` Index Table** |
| **Head Bone Lookup** | Linear string search (`"head_0"`) every frame | **Pre-cached `head_bone_index` on model load** |
| **Win32 IPC Contention** | Synchronous `GetClientRect` every frame | **Throttled window tracking (1 in 15 frames)** |
| **Spectator List UI** | Basic name dump | **Sleek `Spectators ({count})` Header Panel** |
| **Stealth & Safety** | 100% Read-Only / Non-Injected | **100% Read-Only / Non-Injected (Unchanged)** |

---

## ⚡ Key Optimizations

> [!TIP]
> **Synchronized Pawn & Bone LERP**
> CS2 memory updates entity positions at 64 Hz. Projecting raw 64 Hz positions on a 240 Hz monitor causes ESP boxes to jitter. We implemented a **Synchronized Exponential LERP Filter** for player origins and bone matrices in lockstep—delivering silky-smooth ESP movement with zero bone drift.

> [!NOTE]
> **$O(1)$ Entity Resolution & Model Caching**
> Handle lookups use an $O(1)$ hash map, and `head_bone_index` is cached upon model loading—eliminating per-frame string matching across 128 model bones.

> [!IMPORTANT]
> **Win32 Lock Contention Fix**
> Window tracking queries (`GetClientRect`, `ClientToScreen`) are throttled to run once every 15 frames, eliminating graphics render thread stalls.

---

## 🚀 Quick Setup & Usage Guide

### Step 1: Download Valthrun Loader
Download the official Valthrun Loader required to map the read-only kernel driver:
👉 **[Download Valthrun Loader (valth.run/download)](https://valth.run/download)**

### Step 2: Obtain `controller.exe`
- Download `controller.exe` from the latest **[Releases](https://github.com/iarhaaan/valthrun-enhanced/releases)** or compile it locally (see build instructions below).

### Step 3: Launch Sequence
1. Start **Counter-Strike 2**.
2. Go to CS2 **Settings -> Video -> Display Mode** and set it to **"Fullscreen Windowed"** (or **Borderless Windowed**).
3. Open the **Valthrun Loader** and map the read-only memory driver.
4. Place your compiled `controller.exe` inside the **Valthrun Loader directory**.
5. Launch `controller.exe`.
6. Press the **`PAUSE`** key on your keyboard to toggle the interactive overlay settings menu!

---

## 🛠️ Compiling From Source

### Prerequisites
- **Git**: [git-scm.com](https://git-scm.com)
- **Rust Toolchain**: [rustup.rs](https://rustup.rs)
- **Rust Nightly**: Run `rustup toolchain install nightly`

### Build Command
```powershell
# 1. Clone the repository
git clone https://github.com/iarhaaan/valthrun-enhanced.git
cd valthrun-enhanced

# 2. Build optimized release executable
cargo +nightly build --release -p controller
```
The compiled binary will be generated at `target/release/controller.exe`.

---

## 🛡️ Stealth & Anti-Cheat Safety

- **100% Read-Only**: Zero writes to CS2 process memory.
- **Zero Memory Injection**: No DLLs are injected into `cs2.exe`.
- **Standalone DirectX Overlay**: Rendered via an independent, transparent DirectX 11 window overlay (`DwmExtendFrameIntoClientArea`).

---

## 📜 License & Credits

- Original Valthrun project by [Valthrun Team](https://github.com/Valthrun/valthrun-cs2).
- **Valthrun Enhanced** maintained by [iarhaaan](https://github.com/iarhaaan/valthrun-enhanced).
