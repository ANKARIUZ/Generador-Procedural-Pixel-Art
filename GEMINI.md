# GEMINI.md — Project Instructions

## Project
Procedural City Engine: custom procedural 3D generation and rendering system for a SimCity 4-style simulator.

## Core requirements
- Language: Rust.
- No game engines (Godot, Unity, Unreal).
- Graphics API: wgpu.
- Window/input: winit.
- Mathematics: glam.
- Geometry generation must be independent of rendering.
- Objects must be represented as procedural geometry, not static images.
- Rendering style: orthographic, diagonal camera, Pixel Art.
- Random generation must support reproducible seeds.

## Development rules
1. Follow the current sprint in `docs/ROADMAP.md`.
2. Implement only the requested task.
3. Never implement future features without authorization.
4. Preserve existing architecture and working code.
5. Avoid unnecessary dependencies.
6. Do not duplicate existing functionality.
7. Keep modules independent and maintainable.
8. Document important public structures and functions.
9. Test changes and verify compilation.
10. Never claim a test passed without executing it.

## Context management
- Read only the documentation relevant to the current task.
- Do not read all documentation or source files unnecessarily.
- Inspect only the files required for the requested change.
- Reuse existing implementations whenever possible.
- Do not repeat project documentation in responses.

## Required workflow
Before coding:
1. Identify the current sprint.
2. Inspect relevant existing code.
3. Explain the proposed changes briefly.
4. Identify affected files.

After coding:
1. List modified and created files.
2. Explain what changed.
3. Report compilation and test results.
4. Identify remaining issues.

## Restrictions
- Do not change the programming language.
- Do not introduce an external game engine.
- Do not redesign architecture without prior justification and approval.
- Do not advance to another sprint automatically.

## Documentation
- Project vision: `docs/VISION.md`
- Technical architecture: `docs/ARCHITECTURE.md`
- Development plan: `docs/ROADMAP.md`
- Technical decisions: `docs/DECISIONS.md`