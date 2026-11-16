# Development Roadmap — Procedural City Engine

## Development Methodology

The project will follow an incremental Scrum-inspired development process.

Each sprint must have:

- A defined objective.
- A limited implementation scope.
- Verifiable deliverables.
- Compilation and testing.
- A documented completion status.

Do not begin the next sprint until the current sprint has been validated.

---

## Sprint 0 — Project Initialization

**Objective:** Create a functional Rust application with a working window and initialized graphics environment.

Tasks:

- Configure the Cargo project.
- Add winit.
- Add wgpu.
- Initialize the application.
- Create a window.
- Initialize the GPU adapter and device.
- Configure the rendering surface.

Deliverables:

- Rust project compiles.
- Application opens a window.
- wgpu initializes successfully.

Acceptance criteria:

- No critical initialization errors.
- Window opens and remains responsive.
- Application can close correctly.

Status: Completed.

---

## Sprint 1 — Geometry System

**Objective:** Create the fundamental geometric structures.

Tasks:

- Define Vertex.
- Define Face.
- Define Mesh.
- Implement geometric primitives.
- Generate a cube.
- Generate a basic prism.
- Support independent face colors.
- Add geometry tests.

Deliverables:

- Reusable geometry structures.
- Procedurally generated primitives.
- Unit tests.

Acceptance criteria:

- Cube geometry is mathematically correct.
- Vertex and index data are valid.
- Faces can be assigned independent materials or colors.
- Tests pass.

Status: Completed.

Verified results:

- `Vertex`, `Face` y `Mesh` se encuentran en una biblioteca independiente de wgpu.
- El cubo y el prisma rectangular generan ocho vértices compartidos y doce
  triángulos, con materiales independientes por cada lado cuadrilateral.
- La validación rechaza dimensiones no positivas/no finitas, vértices no finitos,
  índices inválidos y triángulos degenerados.
- Las pruebas cubren dimensiones, topología, orientación exterior, materiales,
  determinismo e independencia de la inicialización de GPU.
- Se ejecutaron correctamente formato, compilación, Clippy, pruebas, build y una
  prueba de humo gráfica controlada con `PCE_MAX_FRAMES=1`.

---

## Sprint 2 — Basic Rendering

**Objective:** Render generated geometry using wgpu.

Tasks:

- Create the rendering pipeline.
- Implement vertex buffers.
- Implement index buffers.
- Create basic shaders.
- Render a cube.
- Implement depth buffering.
- Display independent face colors.

Deliverables:

- A rendered cube inside the application window.

Acceptance criteria:

- Cube renders correctly.
- Depth testing works.
- Face colors are distinguishable.
- No invalid GPU buffer operations.

Status: Completed.

Verified results:

- Se convirtió el cubo de `geometry` a buffers de wgpu con vértices expandidos
  por triángulo para conservar los materiales por cara.
- La pipeline WGSL usa colores planos, front face antihorario, culling trasero y
  profundidad `Depth24Plus`.
- La cámara ortográfica diagonal se actualiza ante redimensionamiento sin
  reconstruir recursos geométricos estáticos.
- Las pruebas de CPU verifican conversión, paleta, datos inválidos, vertex layout
  y relación de aspecto. La ventana mostró visualmente las caras roja, verde y
  naranja del cubo; la prueba de un fotograma controlado cerró correctamente.

---

## Sprint 3 — Camera System

**Objective:** Implement the initial camera.

Tasks:

- Implement orthographic projection.
- Configure a diagonal camera.
- Implement camera transformations.
- Add zoom.
- Add camera rotation.
- Support window resizing.

Deliverables:

- Interactive camera system.

Acceptance criteria:

- Camera displays the object from a diagonal perspective.
- Zoom works correctly.
- Rotation works correctly.
- Projection remains correct after resizing.

Status: Completed — Manual interaction and idle/exit validation confirmed by user.

Verified results:

- La cámara orbital mantiene proyección ortográfica, profundidad compatible con
  wgpu, límites de pitch y zoom, y pan sobre el plano XZ dependiente del yaw.
- El controlador procesa ratón, rueda, WASD, R, salida del cursor y pérdida de
  foco sin depender de GPU o ventanas.
- El movimiento continuo usa actualizaciones temporizadas limitadas mientras hay
  teclas mantenidas; la aplicación vuelve a esperar eventos al detenerse.
- Las pruebas matemáticas y de controlador pasan junto con las pruebas previas.
- Falta una validación manual completa de los gestos en ventana antes de marcar
  el Sprint 3 como completado.

---

## Sprint 4 — Modular Geometry and Objects

**Objective:** Build complex objects by combining geometric primitives.

Tasks:

- Implement object transformations.
- Combine multiple meshes.
- Support local coordinates.
- Support object positioning.
- Create reusable geometric components.

Deliverables:

- Modular object construction system.

Acceptance criteria:

- Multiple primitives can form one object.
- Transformations work correctly.
- Geometry remains independent of rendering.

Status: Completed — User confirmed visual scene and controls functioning.

Verified results:

- Se añadieron `Scene`, `SceneObject`, `MeshId`, `ObjectId` y `Transform` como
  componentes CPU independientes de wgpu y winit.
- Las mallas se registran una vez y las instancias las reutilizan mediante
  identificadores estables; una malla no puede retirarse mientras siga en uso.
- El renderizador crea buffers GPU una vez por malla registrada y usa matrices
  de modelo independientes mediante uniforms dinámicos alineados.
- La escena determinista contiene ocho objetos visibles y uno oculto, con cubos
  compartidos, prismas, escalas, posiciones y rotaciones independientes.
- Las pruebas de CPU, la compilación y la prueba de humo de un fotograma pasan.
  Falta validación visual manual completa de la composición y la interacción.

---

## Roadmap revision after Sprint 4 (2026-11-13)

The following milestones supersede the original Sprint 5–7 sequence while preserving its goals: procedural house generation, pixel art rendering, and serialization/export. Future sprint scopes are planned, not implemented. Do not advance automatically.

---

## Sprint 5 — Structural Building Model

**Objective:** Define a GPU-independent building model composed of identifiable architectural components, without implementing destruction yet.

Tasks:
- Component IDs, types, dimensions and validated transforms.
- Material references and basic architectural relationships.
- Separation of building model from `Scene`, GPU resources and damage representation.
- Tests for invariants and composition of components.

Deliverables: Reusable CPU building model and tests.

Acceptance criteria: Components retain stable identities; invalid dimensions/references are rejected; geometry and rendering remain independent.

Status: Completed.

Verified results:

- El módulo CPU `building` reutiliza `scene::Transform` en una sola dirección
  y funciona sin una instancia `Scene`, wgpu, winit, geometría o renderizador.
- Building posee componentes Foundation, Wall, Floor, Column, Roof, Door y
  Window con identidad monotónica, dimensiones finitas positivas, transformación
  validada y referencia de material tipada.
- Las relaciones compatibles se validan sin modificaciones parciales; un
  componente con dependientes no puede eliminarse y ningún identificador se
  reutiliza.
- Las pruebas de dominio verifican registro, unicidad, validación, relaciones,
  errores, determinismo y preservación de invariantes.
- La API del edificio usa `ComponentRef`, que combina identidad de instancia
  e identidad local. Las referencias de otros edificios se rechazan antes de
  consultar o mutar el modelo.

---

## Sprint 6 — Procedural House Generator

**Objective:** Generate a deterministic simple house using the structural model.

Tasks:
- Generate walls, roof, door and windows.
- Configurable dimensions and architectural rules.
- Reproducible random seeds and generation tests.
- Demostrar e inspeccionar el modelo estructural desde consola.

Entregable: una vivienda estructural reproducible como `Building`.

Criterios de aceptación: configuración validada, equivalencia estructural con
la misma semilla y versión, asociaciones válidas y una demostración verificable
sin GPU. La conversión a geometría visual y escena queda pendiente.

Estado: Completado en el alcance estructural, pendiente de validación del usuario.

---

## Sprint 6.1 — Perfil geométrico de cubiertas

Derivar ambas caras de la cubierta, la junta central, los remates a ras y
los cierres de coronación a partir de la estructura del Sprint 6. Verificar
contactos, espesor y límites con pruebas y un ejemplo de consola.

Estado: completado en el alcance de perfiles, pendiente de validación del usuario.

---

## Sprint 7.1 — Conversión de viviendas a mallas 3D

Convertir la vivienda estructural a una envolvente con huecos pasantes,
insertos, cubierta, hastiales y materiales por cara. Conservar la procedencia
CPU e integrar una demostración seleccionable en la aplicación existente.

Estado: implementado. La inspección interactiva completa queda pendiente.

---

## Sprint 7.2 — Renderizado pixel art isométrico

Presentar la vivienda desde una vista isométrica con color y profundidad
a baja resolución, ampliación nearest entera, paleta por material e
iluminación en tres bandas. Conservar el modo clásico para comparación.

Estado: implementado con observaciones. Las pruebas GPU comprueban
ampliación, oclusión y tonos. La revisión de la ventana confirmó el
encuadre, el zoom, el reinicio y el redimensionamiento al maximizar.
Queda por confirmar la órbita, la restauración tras minimizar y la
comparación visual directa con el modo clásico.

---

## Sprint 8 — Exportación e importación de assets

Guardar manifiestos JSON versionados con la estructura, receta y semilla.
Importar con identidades nuevas, regenerar la malla y producir un PNG
isométrico RGBA transparente desde el pase GPU offscreen.

Estado — Implementado. La demostración reconstruyó la vivienda en un proceso
separado y reprodujo el JSON y PNG en el mismo entorno. El ejemplo permite
exportar, importar y volver a dibujar una vivienda. La igualdad de PNG
entre adaptadores o controladores queda fuera del contrato.

---

## Sprint 9 — Hybrid Geometry and Destructible Regions

**Objective:** Prepare optimized surfaces and optional locally subdivided regions for progressive damage.

Tasks:
- Map building components to geometry regions.
- Design local fragments/voxels without dense full-building voxelization.
- Preserve component identity and reuse geometry where possible.
- Define update boundaries and performance tests.

Deliverables: Geometry representation suitable for localized changes.

Acceptance criteria: Unaffected geometry is preserved; regions can be addressed independently; no mandatory full voxel grid.

Status: Planned.

---

## Sprint 10 — Progressive Visual Damage

**Objective:** Represent localized, progressive damage to building components without physical collapse simulation.

Tasks:
- Damage states and deterministic damage application.
- Local visual changes: discoloration, cracks, holes or missing fragments.
- Update affected regions and test reproducibility.
- Preserve the future ability to add structural support relationships.

Deliverables: Demonstration of progressively damaged house geometry/assets.

Acceptance criteria: Localized damage is observable and reproducible; intact regions remain stable; no complete fire or structural physics system is required.

Status: Planned.

---

## Future Development — Urban Simulation

The following features remain outside the initial asset-generator milestone and must not be implemented prematurely. Structural collapse and full fire propagation belong to future simulation work.

### World Generation

- Terrain generation.
- Roads.
- City blocks.
- Neighborhoods.
- Procedural urban layouts.

### Buildings

- Residential buildings.
- Commercial buildings.
- Industrial buildings.
- Public infrastructure.
- Building variations.

### Simulation

- Population.
- Economy.
- Traffic.
- Transportation.
- Public services.
- Environmental systems.

### Dynamic World

- Fire.
- Smoke.
- Structural damage.
- Destruction.
- Reconstruction.
- Weather effects.

### Performance

- Spatial chunking.
- LOD and HLOD.
- Mesh batching.
- GPU instancing.
- Frustum culling.
- Parallel generation.

---

## Sprint Completion Rules

A sprint is considered complete only when:

1. Its implementation tasks are finished.
2. The application compiles.
3. Relevant tests have been executed.
4. Acceptance criteria are verified.
5. Changes are documented.
6. CURRENT_TASK.md is updated.

Do not mark a sprint as completed based only on generated code or an explanation.

The next sprint requires explicit validation of the current one.
