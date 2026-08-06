# ⚡ Valthrun Enhanced CS2

<p align="center">
  <b>An actively maintained, high-performance, ultra-smooth fork of Valthrun CS2.</b><br>
  Engineered for 144Hz, 240Hz, and 360Hz+ high-refresh-rate displays with zero ESP micro-stutter.
</p>

---

## 🔥 Why Valthrun Enhanced is Better Than Original Valthrun

While stock Valthrun provides a solid read-only foundation, default builds suffer from **64 Hz memory tick jitter**, **per-frame string search overhead**, and **Win32 lock contention**. **Valthrun Enhanced** fixes all of these bottlenecks:

| Feature / Metric | Stock Valthrun | Valthrun Enhanced |
| :--- | :--- | :--- |
| **ESP Motion Smoothness** | 64 Hz memory tick jitter | **Silky-Smooth 240Hz+ Synchronized Pawn & Bone LERP** |
| **Entity Lookup Speed** | $O(\log N)$ `BTreeMap` search | **$O(1)$ High-Speed `HashMap` Index Table** |
| **Head Bone Position Lookup** | Linear string search (`"head_0"`) every frame | **Pre-cached `head_bone_index` on model load** |
| **Win32 IPC Contention** | Synchronous `GetClientRect` every frame | **Throttled window tracking (1 in 15 frames)** |
| **Spectator List UI** | Basic name dump | **Sleek `Spectators ({count})` Header Panel** |
| **Anti-Cheat Safety** | 100% Read-Only / Non-Injected | **100% Read-Only / Non-Injected (Unchanged)** |

---

## ⚡ Detailed Optimization Overhaul

### 1. 🎯 Synchronized Pawn & Bone LERP Smoothing
- **The Problem**: CS2 updates player entity positions at server tick rate (**64 Hz** / 15.6ms). Projecting raw 64 Hz memory positions onto a 240 Hz monitor causes ESP boxes and skeletons to jump and stutter.
- **Our Fix**: Built an **Exponential Moving Average LERP Filter** that interpolates both player origins and individual bone positions in 100% lockstep synchronization.
- **Result**: ESP movement transitions smoothly across every rendered frame at **240+ FPS** without bone drift or lag.

### 2. 🚀 $O(1)$ Fast Entity Resolution & Model Bone Caching
- **The Problem**: Stock Valthrun resolves entity handles using a `BTreeMap` and performs string matching (`bone.name == "head_0"`) across all 128 model bones on every single frame.
- **Our Fix**: Upgraded handle resolution to a `$O(1)$ HashMap` and cached `head_bone_index` during model initialization.

### 3. ⚡ Win32 Render Thread Lock Fix
- **The Problem**: Executing synchronous Win32 User APIs (`GetClientRect`, `ClientToScreen`) on every frame causes Windows IPC lock contention, stalling the graphics render thread.
- **Our Fix**: Throttled window boundary queries to run once every 15 frames while reusing cached window coordinates.

---

## 🛡️ Stealth & Anti-Cheat Guarantees

- **100% Read-Only**: We do not write to CS2 process memory in any way.
- **Zero Memory Injection**: No DLLs are injected into `cs2.exe`.
- **Standalone DirectX Overlay**: Rendered via an independent, transparent DirectX 11 window overlay (`DwmExtendFrameIntoClientArea`).

---

## 🛠️ Compilation & Setup Guide

### 1. Prerequisites
1. Install **Git**: [git-scm.com](https://git-scm.com)
2. Install **Rust**: [rustup.rs](https://rustup.rs)
3. Install the **Rust Nightly Toolchain**:
   ```powershell
   rustup toolchain install nightly
   ```

### 2. Cloning the Repository
```powershell
git clone https://github.com/iarhaaan/valthrun-enhanced.git
cd valthrun-enhanced
```

### 3. Building the Release Executable
Run the following cargo command to build the optimized release binary:
```powershell
cargo +nightly build --release -p controller
```

The compiled binary will be generated at:
```
target/release/controller.exe
```

---

## 🎮 How to Use

1. Launch Counter-Strike 2.
2. In CS2 Video Settings, ensure **Display Mode** is set to **"Fullscreen Windowed"** (or **Borderless Windowed**).
3. Run `controller.exe`.
4. Press **`PAUSE`** key on your keyboard to open the Valthrun settings overlay menu.

---

## 📜 License & Credits

- Original Valthrun by [Valthrun Team](https://github.com/Valthrun/valthrun-cs2).
- Valthrun Enhanced maintained by [iarhaaan](https://github.com/iarhaaan/valthrun-enhanced).
