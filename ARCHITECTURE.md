# Valthrun CS2 - Architecture & Codebase Map

This document serves as the authoritative map of the **Valthrun CS2** codebase. Use this guide to quickly locate where specific features, memory readers, overlay graphics, settings, or bugfixes belong.

---

## 1. High-Level Architecture Overview

Valthrun CS2 is a modular, read-only external gameplay enhancement system built in Rust. It consists of six main crate/workspace layers:

```
                  ┌────────────────────────────────────────────────────────┐
                  │                    CS2 Process                         │
                  └──────────────────────────┬─────────────────────────────┘
                                             │ Memory Reads (Kernel / Win32 / DMA)
                                             ▼
                  ┌────────────────────────────────────────────────────────┐
                  │                      cs2 Crate                         │
                  │  (Handles, Entities, Schema Resolver, Memory Offsets)  │
                  └──────────────────────────┬─────────────────────────────┘
                                             │ Game State Snapshots
                                             ▼
                  ┌────────────────────────────────────────────────────────┐
                  │                     utils/state                        │
                  │           (Thread-safe Shared State Sync)              │
                  └──────────────┬──────────────────────────┬──────────────┘
                                 │                          │
                                 ▼                          ▼
 ┌──────────────────────────────────────────────┐  ┌───────────────────────────────┐
 │               controller Crate               │  │          radar Crate          │
 │ (ESP, Bomb, Spectators, Triggerbot, Menu UI) │  │  (WebSocket Broadcast & Web)  │
 └──────────────────────┬───────────────────────┘  └───────────────────────────────┘
                        │ Render Commands
                        ▼
 ┌──────────────────────────────────────────────┐
 │                overlay Crate                 │
 │ (Transparent Window, DirectX/Vulkan/OpenGL)  │
 └──────────────────────────────────────────────┘
```

---

## 2. Feature-to-File Quick Reference

Use this matrix to instantly find where to edit code when fixing or implementing features:

| Feature / System | Key Source File(s) | Description |
| :--- | :--- | :--- |
| **Player ESP (2D/3D Box, Skeleton)** | `controller/src/enhancements/player/mod.rs` | Renders player skeletons, bounding boxes, health bars, head indicators |
| **Player Info Text (Name, Health, Weapon)** | `controller/src/enhancements/player/info_layout.rs` | Layout math for player labels (weapon name, health text, distance) |
| **Bomb Overlay & Defuse Timer** | `controller/src/enhancements/bomb.rs` | Detonation countdown, site location, defuse timer visuals |
| **Triggerbot Logic** | `controller/src/enhancements/trigger.rs` | Crosshair target raycasting, bone hit checks, mouse click actuation |
| **Spectator List Overlay** | `controller/src/enhancements/spectators_list.rs` | Renders list of spectators watching local player or current spec target |
| **Grenade Helper** | `controller/src/enhancements/grenade_helper.rs` | Grenade trajectory rendering and lineup assists |
| **Sniper Crosshair** | `controller/src/enhancements/sniper_crosshair.rs` | Overlay crosshair while scoped or wielding sniper rifles |
| **Aim Assistant** | `controller/src/enhancements/aim.rs` | Aim target tracking logic |
| **3D to 2D Screen Projection** | `controller/src/view/world.rs` | `world_to_screen` coordinate translation using view matrix |
| **Settings Menu UI (Egui)** | `controller/src/settings/ui.rs` | Interactive options overlay (Pause key menu layout) |
| **ESP Configuration & Colors** | `controller/src/settings/esp.rs` | ESP toggles, colors, line widths, visibility settings struct |
| **App Configuration Storage** | `controller/src/settings/config.rs` | Save/load user settings to/from config files |
| **App Main & Memory Backend Init** | `controller/src/main.rs` | Memory provider initialization (Win32/KVM/PCILeech), main loop |
| **Memory Reader Handle** | `cs2/src/handle.rs` | Safe memory reading API (`read_struct`, `read_array`, `read_string`) |
| **Static Offsets & Signatures** | `cs2/src/offsets.rs` | Offsets (`dwEntityList`, `dwViewMatrix`, `dwLocalPlayerPawn`) |
| **Entity List Reader** | `cs2/src/entity/list.rs` | CS2 entity list iteration and entity lookup |
| **Player Entity State Data** | `cs2/src/state/player.rs` | Health, armor, team, bone positions, view angles, visibility |
| **Bomb Entity State Data** | `cs2/src/state/bomb.rs` | `PlantedC4` memory parsing, timer calculation, defuse status |
| **Observer State Reader** | `cs2/src/state/observer.rs` | Spectator tracking memory reader |
| **Source 2 Schema Resolver** | `cs2/src/schema_gen.rs` | Runtime dynamic schema scanner resolving field offsets |
| **DirectX 11 Overlay Renderer** | `overlay/src/directx/` | DX11 graphics pipeline implementation for overlay |
| **Vulkan Overlay Renderer** | `overlay/src/vulkan/` | Vulkan graphics pipeline implementation |
| **OpenGL Overlay Renderer** | `overlay/src/opengl/` | OpenGL graphics pipeline implementation |
| **Input Hook / Window Passthrough** | `overlay/src/input.rs` | Mouse/keyboard event capture & click-through window behavior |
| **Game Window Tracker** | `overlay/src/window_tracker.rs` | Syncs overlay size & location with CS2 window boundaries |
| **CS2 Schema Dumper** | `cs2-schema/dumper/` | Dumps CS2 schema definitions directly from process DLLs |
| **CS2 Engine Data Structures** | `cs2-schema/cutl/` | CUtlVector, CUtlTSHash implementations |
| **Web Radar Broadcast Client** | `radar/client/` | Sends player positions from CS2 state to radar server |
| **Web Radar Server** | `radar/server/` | Hosts WebSocket server & static web radar app |
| **Web Radar Web App** | `radar/web/` | React/TypeScript web app for map display |

---

## 3. Crate Breakdown & Detail

### 3.1 `controller` (Enhancements, Menu UI, Main Loop)
- **`src/main.rs`**: Main binary entry point. Coordinates memory reader provider (Win32 API fallback, KVM, PCILeech DMA, Kernel IOCTL driver), initializes overlay, polls hotkeys, runs game ticks.
- **`src/enhancements/`**: Contains all visual overlays & gameplay assistance logic.
  - `player/mod.rs` & `player/info_layout.rs`: All ESP visual features.
  - `bomb.rs`: Bomb site, C4 detonation timer, defuse progress.
  - `trigger.rs`: Crosshair triggerbot.
  - `spectators_list.rs`: Observer target list.
  - `grenade_helper.rs` & `sniper_crosshair.rs`: Additional visual helpers.
- **`src/settings/`**:
  - `ui.rs`: The Egui menu rendered when pressing `PAUSE`.
  - `esp.rs`, `config.rs`, `hotkey.rs`: Settings models & persistence.
- **`src/view/`**:
  - `world.rs`: World-to-screen matrix transforms.

### 3.2 `cs2` (CS2 Memory Reading & Schema Engine)
- **`src/handle.rs`**: Core process memory reader interface (`ProcessHandle`).
- **`src/offsets.rs`**: Base module offsets (`dwEntityList`, `dwViewMatrix`, etc.).
- **`src/entity/`**: Raw entity controllers (`controller.rs`, `list.rs`, `identity.rs`).
- **`src/state/`**: Parsed game state.
  - `player.rs`: Player struct containing positions, health, bones, view angles, weapon.
  - `bomb.rs`: Planted C4 state.
  - `observer.rs`: Spectators state.
  - `map.rs`: Current map name.
  - `globals.rs`: Tick count and global variables.
- **`src/schema_gen.rs` & `src/schema_runtime/`**: Dynamic schema offset solver.

### 3.3 `overlay` (Transparent GPU Overlay Window)
- **`src/lib.rs`**: Creates top-most transparent window & hooks render loop.
- **`src/directx/`**, **`src/vulkan/`**, **`src/opengl/`**: Hardware rendering backends.
- **`src/input.rs`**: Handles input passthrough (click-through when menu closed, captures input when menu open).
- **`src/window_tracker.rs`**: Position/resolution tracking of `cs2.exe`.

### 3.4 `cs2-schema` (Source 2 Schema Definitions)
- **`dumper/`**: Schema dumper tool.
- **`cutl/`**: Source 2 container structures (`CUtlVector`, `CUtlTSHash`).
- **`generated/cs2_schema.json`**: Pre-dumped CS2 schema catalog.

### 3.5 `radar` (Web Radar)
- **`shared/`**: JSON protocol for player coordinates.
- **`client/`**: Broadcasts player locations over WebSocket.
- **`server/`**: Hosts WebSocket hub & web UI.
- **`web/`**: React/TS web app for rendering interactive top-down radar.

### 3.6 `utils/state` (Thread Synchronization)
- **`src/lib.rs`**: Thread-safe `StateContainer` passing game snapshots between memory thread, overlay thread, and radar thread.
