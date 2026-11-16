# Project Vision — Procedural City Engine

## 1. Project Overview

Procedural City Engine is a custom urban simulation technology designed to create a city-building game inspired by SimCity 4.

The project will be developed entirely in Rust, using a custom rendering and procedural generation architecture.

The objective is to build a system capable of generating, rendering, and simulating urban environments using mathematical geometry, modular objects, and procedural algorithms.

The project will not use external game engines such as Unity, Unreal Engine, or Godot.

## 2. Main Objectives

- Develop a custom 3D rendering system using wgpu.
- Generate buildings and urban objects procedurally.
- Represent objects using geometric meshes rather than static images.
- Support independent materials and colors for individual faces.
- Implement an orthographic diagonal camera.
- Create a controlled Pixel Art visual style.
- Support reproducible procedural generation using random seeds.
- Build an architecture capable of scaling to large urban environments.

## 3. Visual Direction

The game will use a diagonal, elevated camera perspective.

The visual style will combine three-dimensional geometry with a Pixel Art appearance.

The rendering system should support:

- Orthographic projection.
- Camera zoom and rotation.
- Low-resolution rendering.
- Nearest-neighbor image scaling.
- Controlled color palettes.
- Face-based lighting.
- Shadows and visual outlines.
- Different levels of geometric detail.

The final appearance should resemble a three-dimensional world rendered with a consistent Pixel Art aesthetic.

## 4. Procedural Generation

Objects will be constructed from mathematical geometry.

Buildings may consist of multiple geometric components, including:

- Walls.
- Roofs.
- Doors.
- Windows.
- Balconies.
- Chimneys.
- Decorative elements.

Generation must support configurable parameters and reproducible random seeds.

The same seed and configuration should produce the same generated object.

## 5. Simulation Vision

The long-term objective is to create an urban simulation containing:

- Buildings and residential areas.
- Roads and transportation.
- Vehicles.
- Citizens and population.
- Public services.
- Economic systems.
- Environmental conditions.
- Fires and structural damage.
- Destruction and reconstruction.

Simulation logic must remain independent of rendering logic.

Objects should continue to exist and evolve as simulation data, even when they are outside the camera view.

## 6. Performance Goals

The architecture must be designed for large environments.

Potential optimization techniques include:

- Level of Detail (LOD).
- Hierarchical Level of Detail (HLOD).
- Spatial chunking.
- Mesh batching.
- GPU instancing.
- Frustum culling.
- Occlusion techniques.
- Parallel procedural generation.

These techniques should be introduced when required by the project's development stage.

## 7. Development Philosophy

The project will be developed incrementally.

Each sprint must produce a functional and verifiable result.

The architecture should prioritize:

- Modularity.
- Maintainability.
- Performance.
- Reproducibility.
- Separation of responsibilities.
- Technical clarity.

Do not implement future systems before the current development stage has been validated.

## 8. Initial Scope

The first milestone is a minimal graphical application capable of:

1. Opening a window.
2. Initializing wgpu.
3. Generating a geometric cube.
4. Rendering the cube with different face colors.
5. Displaying the object using an orthographic diagonal camera.
6. Supporting camera zoom and rotation.
7. Generating a simple procedural house.

The first milestone does not include the complete city simulation.