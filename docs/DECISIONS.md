# Technical Decisions

## DEC-001: Programming language
Decision: Rust.
Status: Accepted.

## DEC-002: Graphics API
Decision: wgpu.
Status: Accepted.

## DEC-003: Game engine
Decision: No external game engine.
Status: Accepted.

## DEC-004: Procedural geometry
Decision: Generate objects using mathematical geometry and modular meshes.
Status: Accepted.

## DEC-005: Rendering architecture
Decision: Procedural generation and rendering must remain independent.
Status: Accepted.

## DEC-006: Camera
Decision: Orthographic diagonal camera.
Status: Accepted.

## DEC-007: Visual style
Decision: Pixel Art rendering with controlled resolution and nearest-neighbor scaling.
Status: Accepted.

## DEC-008: Atributos de renderizado por cara
Decision: Expandir los vértices por triángulo en el límite `render → geometry`
para asociar de forma inequívoca el color del material de cada cara, sin añadir
atributos específicos de GPU a `geometry::Vertex`.
Status: Accepted.

## DEC-009: Pipeline inicial de demostración
Decision: Usar cámara ortográfica diagonal fija, profundidad `Depth24Plus`,
front face CCW y culling trasero para el cubo del Sprint 2.
Status: Accepted.

## DEC-010: Cámara orbital separada del controlador
Decision: Mantener el estado y matemáticas de cámara en `Camera`, interpretar
entradas abstractas en `CameraController` y dejar a `RenderContext` limitado a
actualizar el uniform de GPU.
Status: Accepted.

## DEC-011: Movimiento continuo bajo demanda
Decision: Mantener `ControlFlow::Wait` cuando la cámara está inactiva y utilizar
`WaitUntil` limitado a 16 ms solo mientras WASD permanece presionado.
Status: Accepted.

## DEC-012: Escena propietaria y uniformes dinámicos por instancia
Decision: `Scene` es propietaria de las mallas CPU y entrega identificadores
monotónicos para mallas y objetos. Los objetos solo conservan `MeshId`,
`Transform` y visibilidad. El renderizador mantiene un `GpuMesh` por `MeshId` y
usa un buffer uniforme de modelos con offsets dinámicos alineados para dibujar
instancias independientes dentro de la misma submission.
Justification: evita duplicar geometría por objeto y evita la ambigüedad de
escribir repetidamente un único uniform antes de varios draws.
Status: Accepted.


## DEC-013: Enfoque del producto
Decision: Priorizar un generador procedural de assets urbanos pixel art en Rust. Cámara, escena y renderizado son herramientas de creación e inspección; no desarrollar todavía el videojuego completo.
Justification: Mantener el alcance y la modularidad del proyecto.
Status: Accepted.

## DEC-014: Representación arquitectónica híbrida
Decision: Separar modelo arquitectónico, estado de daño y representación visual. Usar geometría optimizada y subdivisión voxel/fragmentos solo en regiones que la necesiten, sin exigir un cubo por unidad de volumen.
Justification: Compatibilizar detalle destructible y rendimiento.
Status: Accepted.

## DEC-015: Destrucción progresiva por etapas
Decision: Diseñar componentes con identidad estable y capacidad futura de relaciones de soporte. Implementar primero daño visual progresivo localizado; posponer estabilidad estructural, colapsos y simulación física.
Justification: Permitir evolución sin implementar prematuramente un sistema de física.
Status: Accepted.

## DEC-016: Separación de materiales y simulación
Decision: Separar propiedades visuales de las propiedades de comportamiento de materiales. El generador produce estructura, representaciones y metadatos; el futuro juego decidirá eventos y simulará incendios, propagación y colapsos.
Justification: Evitar acoplar el generador a mecánicas de juego.
Status: Accepted.

## DEC-017: Reproducibilidad y exportación
Decision: Planificar generación determinista mediante semilla, parámetros y versión de algoritmo; exportar imágenes pixel art y, cuando sea necesario, estructura, IDs de componentes y metadatos. Los formatos se decidirán durante sus respectivos sprints.
Justification: Reutilización y compatibilidad futura de assets.
Status: Accepted.

## DEC-018: Modelo arquitectónico independiente de representación
Decision: Representar los edificios mediante un agregado Building propietario
de componentes arquitectónicos tipados, con identificadores monotónicos,
dimensiones, Transform existente, referencias de material y relaciones
validadas de padre a hijo.
Justification: establece una estructura reutilizable para generación, daño y
exportación futuros sin acoplarla a mallas, recursos GPU, simulación o índices
de contenedores.
Status: Accepted.

El modelo reutiliza `scene::Transform` mediante una dependencia unidireccional y
funciona sin una instancia `Scene`, wgpu ni winit. La identidad local de
`ComponentId` se conserva y su uso en la API de acceso evoluciona en DEC-019.

## DEC-019: Procedencia de referencias de componentes

`Building::new()` asigna un `BuildingId` mediante un contador atómico
comprobado. El orden relajado basta para reservar valores distintos entre hilos.
El constructor informa agotamiento y el contador nunca se reinicia durante el
proceso. `Building` carece de clonación y construcción por defecto.

`ComponentRef` une ese ID de instancia con el `ComponentId` monotónico local.
Las operaciones verifican el ID de instancia antes de resolver el componente.
La identidad persistente de un asset deberá definirse en su formato futuro.
Al cargarlo se asignará un nuevo ID de instancia y se reconstruirán sus
referencias. La semilla y la configuración determinan la estructura generada
con independencia de la identidad de instancia.

## DEC-020: Generación estructural versionada de viviendas

La versión 1 genera un `Building` mediante su API pública y valida capacidad
y márgenes antes de construirlo. SplitMix64 local fija el flujo pseudoaleatorio
sin añadir dependencias. La semilla modifica la distribución de ventanas,
mientras la configuración determina el tamaño y los materiales.
El techo se describe con dos componentes inclinados y las aberturas mediante
componentes asociados a sus paredes. La conversión a mallas y a escena queda
pendiente de una etapa posterior.

## DEC-021: Cubierta derivada con junta vertical y espesor normal

La capa geométrica deriva dos aguas con cortes verticales y límites a ras.
El plano inferior arranca en el borde exterior de la coronación y el plano
superior conserva el espesor normal. Los hastiales y rellenos laterales se
derivan bajo ese plano en bandas disjuntas. La junta de cumbrera y las
superficies de contacto son interfaces internas que el futuro mallador
deberá excluir de la envolvente. El adaptador consume el modelo arquitectónico
y la matemática del perfil permanece independiente de `Building`.

## DEC-022: Envolvente de vivienda por diferencias de ocupación

El mallador divide la planta en franjas rectangulares y deriva bandas sólidas
de paredes, huecos y perfiles de cubierta. La frontera se triangula con
orientación exterior y excluye las superficies de contacto entre sólidos.
Los insertos retranqueados preservan el hueco de pared y se agrupan para GPU
con la envolvente. La procedencia por cara se conserva mediante `ComponentRef`.

## DEC-023: Presentación pixel art por escala entera

La escena conserva su representación 3D e introduce un destino de color a
baja resolución con su propio adjunto de profundidad. Un pase nearest
amplía cada téxel a un bloque cuadrado de píxeles físicos. El viewport
centrado absorbe los sobrantes mediante bandas negras. La presentación
clásica sigue disponible para comparar desde la misma pose de la casa.

El estilo combina una paleta de presentación ligada a los IDs materiales y
tres bandas de luz direccional fija. Se conserva el movimiento continuo de
la cámara ortográfica. El ajuste a cuadrícula para capturas queda pendiente
de una etapa posterior.

## DEC-024: Manifiesto estructural y sprite isométrico

El esquema 1 conserva componentes y relaciones documentales como fuente de
verdad. La semilla y la receta versionada permiten regenerar por separado.
Importar asigna identidades nuevas al reconstruir el modelo mediante su API
pública. El nombre lógico identifica el asset dentro de un catálogo.

La exportación reutiliza el pase 3D pixel art sin una superficie de escritorio.
El destino RGBA sRGB se limpia con alfa cero y las caras tienen alfa uno.
El PNG conserva alfa recto y se amplía con replicación entera. El manifiesto
es el último archivo publicado y marca el paquete completo.
