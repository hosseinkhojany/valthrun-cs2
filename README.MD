<div align="center">

  <h1>⚡ Valthrun Enhanced CS2</h1>

  <p>
    <b>The smoothest external read-only CS2 overlay ever built.</b><br>
    Snapshot interpolation, a dedicated memory-reader thread, and a zero-allocation render path.<br>
    Engineered for 144Hz, 240Hz, and 360Hz+ displays.
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

**Valthrun Enhanced** is an open-source, read-only external gameplay overlay for Counter-Strike 2, forked from the excellent [Valthrun](https://github.com/Valthrun/valthrun-cs2) project.

Stock Valthrun reads game memory and renders ESP **on the same thread, at the same moment**. Since CS2 only updates entity positions at **64 Hz**, projecting those raw positions onto a 240Hz+ monitor produces visible stepping and micro-stutter — and every driver read happens *inside* your render frame, so any kernel latency spike becomes a visible hitch.

**Valthrun Enhanced rebuilds the entire data path** to fix this at the architecture level, while remaining 100% read-only and non-injected.

---

## 🆚 Why Valthrun Enhanced is Better

| System | Stock Valthrun | Valthrun Enhanced ⚡ |
| :--- | :--- | :--- |
| **ESP Motion** | Raw 64 Hz positions rendered per frame → visible stepping | **Snapshot Interpolation Engine** — renders *between* game states for mathematically linear motion |
| **Memory Reads** | Synchronous driver reads inline on the render thread | **Dedicated 300 Hz background reader thread** — the render thread never touches the driver |
| **Frame Pacing** | Fixed per-frame LERP (behaves differently at 144 vs 240 FPS) | **Time-based interpolation** — frame-rate independent at any refresh rate |
| **Read→Render Handoff** | Same thread (blocking) | **Lock-free `ArcSwap` double-buffered snapshots** — zero locks, zero copies on the hot path |
| **Hot Path Allocations** | Deep-clones bone matrices per player per frame | **Recycled buffers** — near-zero allocations while rendering |
| **Entity Lookup** | $O(\log N)$ `BTreeMap` search | **$O(1)$ `HashMap` index table** |
| **Head Bone Lookup** | Linear `"head_0"` string search every frame | **Pre-cached `head_bone_index` on model load** |
| **Win32 IPC** | Synchronous `GetClientRect` every frame | **Throttled window tracking (1 in 15 frames)** |
| **Observability** | FPS counter | **Frame-time 1% low, reads/s, tick cadence, snapshot age, reader rate — live in the overlay** |
| **Stealth & Safety** | 100% Read-Only / Non-Injected | **100% Read-Only / Non-Injected (unchanged)** |

---

## 🧠 The Zero-Lag Architecture

```
┌─────────────────────┐   fixed-rate poll (up to 500Hz)   ┌──────────────────────┐
│  Memory Reader      │ ─────────────────────────────────▶│  Lock-free snapshot   │
│  Thread             │        immutable EspSnapshot      │  swap (ArcSwap)       │
│  (all driver reads) │                                   │                       │
└─────────────────────┘                                   └──────────┬───────────┘
                                                                     │ latest
                    ┌────────────────────────────────────────────────▼──────────┐
                    │  Render thread (uncapped, zero driver reads)              │
                    │  • Per-entity ring buffer of timestamped snapshots        │
                    │  • Renders at now − adaptive delay, linear interpolation  │
                    │  • Origin + all 128 bones interpolated in lockstep        │
                    └───────────────────────────────────────────────────────────┘
```

> [!TIP]
> **Snapshot Interpolation ("Silky" mode — default)**
> The overlay records timestamped snapshots of every player's origin and bone matrices, then renders slightly in the past, interpolating *between* the two nearest game states. The 64 Hz memory staircase becomes perfectly linear motion. The delay **auto-tunes itself** to the measured game tick cadence (typically ~20ms), so there is nothing to configure.

> [!NOTE]
> **"Zero-Delay" mode**
> Prefer zero added latency over perfect linearity? Switch the smoothing mode and the engine extrapolates from the newest snapshot using real velocity — clamped so it can never visibly overshoot.

> [!IMPORTANT]
> **Why the reader thread matters**
> Every driver read is a kernel transition. On stock Valthrun, those happen *inside* your render frame — any spike is a visible hitch. In Valthrun Enhanced, the render thread only performs an atomic pointer swap. Driver latency can no longer stall a single rendered frame.

> [!CAUTION]
> **Teleport & respawn safety**
> A >100-unit single-tick jump flushes an entity's interpolation history instantly — no smearing across the map on respawns, teleports, or entity-ID recycling. Smoothing can also be disabled entirely in settings, restoring stock behavior.

---

## 🎛️ Smoothing Settings (ESP → Smoothing)

| Setting | Default | What it does |
| :--- | :--- | :--- |
| **Enable ESP smoothing** | `On` | Master switch. Off = raw stock behavior. |
| **Mode** | `Silky` | `Silky` interpolates (smoothest). `Zero-Delay` extrapolates (lowest latency). |
| **Adaptive delay** | `On` | Matches render delay to the measured game tick rate automatically. |
| **Interpolation delay** | `10ms` | Manual delay when adaptive is off. |
| **Max extrapolation** | `24ms` | Cap on velocity projection into the future. |
| **Reader poll rate** | `300 Hz` | Background thread read rate. Anything above ~128 Hz is freshness headroom (the game ticks at 64 Hz) — lower it if you want fewer reads/s. |

Live diagnostics are shown directly below the settings: **tick cadence, effective delay, tracked players, snapshot age, and reader rate** — plus a **frame-time 1% low** readout in the watermark, so smoothness is measured, not guessed.

---

## 🚀 Quick Setup & Usage Guide

### Step 1: Download Valthrun Loader
Download the official Valthrun Loader required to map the read-only kernel driver:
👉 **[Download Valthrun Loader (valth.run/download)](https://valth.run/download)**

### Step 2: Obtain `controller.exe`
- Download `controller.exe` from the latest **[Releases](https://github.com/iarhaaan/valthrun-enhanced/releases)** or compile it locally (see build instructions below).

### Step 3: Launch Sequence
1. Start **Counter-Strike 2**.
2. Go to CS2 **Settings → Video → Display Mode** and set it to **"Fullscreen Windowed"** (or **Borderless Windowed**).
3. Open the **Valthrun Loader** and map the read-only memory driver.
4. Place your compiled `controller.exe` inside the **Valthrun Loader directory**.
5. Launch `controller.exe`.
6. Press the **`PAUSE`** key on your keyboard to toggle the interactive overlay settings menu!

---

## ✨ Feature Set

- **Player ESP** — 2D/3D boxes, skeleton with per-bone interpolation, health bars, head dot, tracers, off-screen arrows
- **Player info** — name, weapon, ammo, HP, distance, grenades, flags (kit / bomb carrier / scoped / flashed)
- **Bomb tools** — detonation timer with defuse tracking, world-space bomb label (planted + dropped)
- **Triggerbot** — randomized delay window, team check, post-delay target retest
- **Recoil helper** — simple aim-punch compensation
- **Spectator list** — see who's watching you, in a clean header panel
- **Grenade helper** — per-map lineup spots with position & angle guidance
- **Sniper crosshair** — overlay dot while holding a sniper rifle
- **Stream-proof overlay** — hidden from screen capture (optional)

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

- **100% Read-Only**: Zero writes to CS2 process memory. The interpolation engine and reader thread only ever *read*.
- **Zero Memory Injection**: No DLLs are injected into `cs2.exe`.
- **Standalone DirectX Overlay**: Rendered via an independent, transparent DirectX 11 window overlay (`DwmExtendFrameIntoClientArea`).
- **Optional screen-capture exclusion**: `SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)`.

---

## 📜 License & Credits

- Original Valthrun project by the [Valthrun Team](https://github.com/Valthrun/valthrun-cs2).
- **Valthrun Enhanced** maintained by [iarhaaan](https://github.com/iarhaaan/valthrun-enhanced).
