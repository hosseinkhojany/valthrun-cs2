# Valthrun CS2 Agent Rule - Codebase Architecture Reference

When working on Valthrun CS2, always consult `ARCHITECTURE.md` to locate the exact crates and source files for features, memory readers, settings, and renderers.

## Quick File Locations:
- **Player ESP (2D/3D Box, Skeleton)**: `controller/src/enhancements/player/mod.rs`
- **Bomb Overlay & Defuse Timer**: `controller/src/enhancements/bomb.rs`
- **Triggerbot**: `controller/src/enhancements/trigger.rs`
- **Spectator List**: `controller/src/enhancements/spectators_list.rs`
- **World-to-Screen Matrix Projection**: `controller/src/view/world.rs`
- **Settings Menu UI (Egui)**: `controller/src/settings/ui.rs`
- **Game Memory Handle API**: `cs2/src/handle.rs`
- **Static Memory Offsets**: `cs2/src/offsets.rs`
- **Player State & Bones**: `cs2/src/state/player.rs`
- **Schema Resolver**: `cs2/src/schema_gen.rs`
- **Overlay GPU Renderers**: `overlay/src/directx/`, `overlay/src/vulkan/`, `overlay/src/opengl/`
- **Input Hook & Click-Through**: `overlay/src/input.rs`
- **Web Radar**: `radar/client/`, `radar/server/`, `radar/web/`
