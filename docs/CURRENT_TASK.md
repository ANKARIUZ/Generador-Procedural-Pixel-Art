# Current Task

Sprint actual — 8, exportación e importación de assets

Objetivo actual — Exportar viviendas como JSON estructural y PNG pixel art
transparente. Importar la estructura y volver a renderizarla sin ventana.

Estado — Implementado. El ciclo de exportación e importación se verificó
en procesos separados.

Completed:
- Corregido el bucle de eventos para usar `ControlFlow::Wait` y renderizado bajo
  demanda; el redibujado ocurre al iniciar, redimensionar o recuperar la superficie.
- `PCE_MAX_FRAMES` mantiene una secuencia de fotogramas finita exclusivamente
  para pruebas de humo, sin activar renderizado continuo en ejecución normal.
- Los fallos de creación de ventana e inicialización de wgpu se registran con
  contexto y se propagan desde `App` hasta `main` como errores no exitosos.
- Añadida la biblioteca `geometry`, independiente de wgpu, con `Vertex`, `Face`,
  `Mesh`, validación explícita y primitivas `Cube` y `RectangularPrism`.
- Establecidas las convenciones de mano derecha, Y vertical, Z hacia delante y
  triángulos antihorarios vistos desde el exterior.
- Verificados `cargo fmt --check`, `cargo check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test`, `cargo build` y una ejecución gráfica con `PCE_MAX_FRAMES=1`.
- Reforzada la validación de triángulos: índices repetidos, límites, vértices no
  finitos, degeneración algebraica y resultados geométricos no finitos se
  distinguen mediante errores específicos.
- `add_vertex` usa una conversión comprobada a `u32` y rechaza el primer índice
  no representable sin modificar la malla.
- `face_normal` ahora devuelve `Result<Vec3, GeometryError>` y nunca indexa los
  vértices sin comprobarlos; las mallas públicas corrompidas informan un error.
- `add_quad` valida y acepta solo cuadriláteros simples, coplanares y convexos;
  las dos caras se insertan de forma atómica después de validar toda la entrada.
- Ampliadas las pruebas a 18 casos geométricos en total (16 de integración y 2
  unitarias de límites de índice), incluida la seguridad ante una malla
  modificada tras su validación.
- Implementada la conversión de `Mesh` a datos GPU con vértices expandidos por
  triángulo, preservando el color que corresponde al `material_id` de cada cara.
- Añadidos buffers estáticos de vértices/índices, shader WGSL, pipeline con
  front face CCW, culling trasero y buffer de profundidad `Depth24Plus`.
- Añadida una cámara diagonal ortográfica temporal; la proyección y profundidad
  se actualizan al redimensionar sin reconstruir la geometría ni los buffers.
- Añadidas 7 pruebas de CPU del renderizador para conversión, paleta, mallas
  modificadas, layout GPU y proyección; el total del proyecto es 26 pruebas.
- Verificado visualmente el cubo: caras frontal roja, derecha verde y superior
  naranja visibles simultáneamente en una ventana real.
- Reemplazada la cámara fija por `Camera`, una cámara orbital ortográfica
  independiente de GPU con estado de objetivo, yaw, pitch, distancia, escala y
  relación de aspecto.
- Añadido `CameraController`, independiente de winit, para traducir arrastre
  izquierdo, rueda, arrastre central, WASD, R, salida del cursor y pérdida de foco.
- `RenderContext` actualiza únicamente el uniform de cámara cuando cambia el
  estado; no recrea buffers, pipeline, shader ni profundidad durante interacción.
- WASD usa `WaitUntil` a 16 ms solo mientras hay movimiento; en reposo vuelve a
  `ControlFlow::Wait`, y las repeticiones de teclado no reinician el delta temporal.
- Añadidas 12 pruebas de cámara y controlador; el total del proyecto es 38 pruebas.
- Añadido el módulo opcional `diagnostics`, aislado tras la característica Cargo
  homónima. Registra exclusivamente presentaciones exitosas y expone FPS, tiempo
  de CPU por fotograma, total acumulado y estado `ACTIVE`/`IDLE`.
- El título se refresca aproximadamente una vez por segundo sin solicitar un
  redibujado. En reposo usa un vencimiento ligero y, con WASD, lo combina con el
  vencimiento de movimiento de 16 ms sin alterar la política de renderizado.
- Añadidas pruebas deterministas de conteo, FPS, transición a reposo, ausencia de
  tiempo de fotograma sin presentación y formato del título.
- Añadido `scene`, independiente de wgpu y winit, con propiedad de mallas CPU,
  objetos identificados de forma monotónica, visibilidad y referencias `MeshId`.
- Añadido `Transform` con traslación, cuaternión normalizado y escala positiva;
  valida valores no finitos, cuaterniones nulos, escalas negativas, cero y
  subnormales, y compone `Translation × Rotation × Scale`.
- El renderizador almacena un recurso GPU por malla registrada y selecciona una
  matriz de modelo por objeto mediante offsets dinámicos alineados; no duplica
  buffers de geometría por instancia ni los reconstruye por fotograma.
- La escena demostrativa tiene ocho objetos visibles y uno oculto; incluye cubos
  reutilizados, prismas de distintas alturas, transformaciones independientes y
  un encuadre inicial ortográfico ampliado para abarcarla.
- Añadidas 6 pruebas CPU de escena y transformaciones. Se conserva renderizado
  bajo demanda, controles de cámara, profundidad, culling y diagnóstico opcional.
- El módulo `building` reutiliza `scene::Transform` en una sola dirección y funciona
  sin una instancia `Scene` y sin dependencias de `geometry`, wgpu o winit.
  `Building` posee componentes Foundation, Wall, Floor, Column, Roof, Door y
  Window con IDs monotónicos y estables dentro de su edificio.
- Los componentes validan dimensiones positivas y
  finitas, y exponen una referencia tipada de material preparada para futuras
  propiedades visuales, térmicas, de resistencia y combustibilidad.
- Se validan relaciones arquitectónicas: puertas y ventanas pertenecen a paredes;
  techos a paredes o columnas; pisos, paredes y columnas a soportes compatibles.
  No se permiten referencias inexistentes, relaciones incompatibles ni eliminar
  componentes con dependientes.
- El modelo de edificio cuenta con pruebas independientes de GPU.
- S5-01 quedó resuelto con `BuildingId` único por proceso y `ComponentRef` obligatorio
  en las operaciones que identifican componentes. Los IDs locales pueden
  coincidir entre edificios y siguen siendo deterministas. Las referencias
  cruzadas se rechazan sin mutar los componentes, relaciones o contadores.
- Los fotogramas omitidos por tamaño cero quedan fuera del conteo de diagnósticos
  y de `PCE_MAX_FRAMES`. Al restaurar el tamaño, se solicita un nuevo dibujo.
- `procedural` genera una vivienda con fundación, piso, cuatro paredes, puerta,
  ventanas configurables y dos aguas estructurales. El ejemplo de consola
  imprime la semilla, los componentes y sus relaciones.

Advertencias del entorno pendientes:
- Cargo no puede crear hard links en la caché incremental y copia archivos como
  alternativa. No afecta la corrección ni el binario; solo puede ralentizar la compilación.
- Falta la capa opcional `VK_LAYER_KHRONOS_validation`; wgpu y el cargador Vulkan
  también informan búsquedas de manifiestos de capas. La AMD Radeon se inicializa
  correctamente con Vulkan. No se modificaron controladores, registro ni validaciones.

Antecedente de Sprint 7.2 — El informe anterior dejó como comprobaciones
manuales la órbita, la restauración tras minimizar y la comparación con el
modo clásico. El Sprint 8 añadió una verificación independiente de arranque
y presentación en ambos modos mediante pruebas de humo de dos fotogramas.

Siguiente tarea — Validación del Sprint 8 por el usuario. El Sprint 9
permanece planificado.

Last Verified:
El 2026-11-13 se ejecutaron correctamente formato, check, Clippy estricto,
pruebas y build con y sin diagnostics: 49 pruebas en la configuración base y
53 con diagnóstico. Las pruebas de humo con PCE_MAX_FRAMES=1 inicializaron Vulkan
sobre la AMD Radeon, presentaron un fotograma y cerraron correctamente en ambas
configuraciones.

La corrección de S5-01 pasó formato, check, Clippy estricto, pruebas y build.
Pasaron 56 pruebas base y 60 con `diagnostics`, además de tres pruebas de
compilación negativa en cada configuración. Ambas variantes presentaron dos
fotogramas sobre Vulkan y cerraron con `PCE_MAX_FRAMES=2`.

En el Sprint 6 pasaron formato, check, Clippy estricto, pruebas y build en
ambas configuraciones. Pasaron 61 pruebas base y 65 con `diagnostics`,
además de tres pruebas de compilación negativa en cada configuración.
`cargo run --example house` mostró 12 componentes y sus relaciones con
semilla 42. El ejecutable gráfico presentó dos fotogramas sobre Vulkan y
cerró normalmente con `PCE_MAX_FRAMES=2`, con y sin diagnóstico.

En Sprint 6.1 pasaron formato, check, Clippy estricto, pruebas y build.
Pasaron 68 pruebas base y 72 con `diagnostics`, más tres pruebas de
compilación negativa en cada variante. Los ejemplos `house` y `roof`
mostraron la estructura y las secciones derivadas. Ambas pruebas de humo
presentaron dos fotogramas sobre Vulkan y cerraron normalmente.

En Sprint 7.1 pasaron formato, check, Clippy estricto y build en ambas
configuraciones. Pasaron 73 pruebas base y 77 con `diagnostics`, más
tres pruebas de compilación negativa en cada variante. Los ejemplos
`house` y `roof` funcionaron. Las pruebas de humo de la escena clásica
y de la vivienda con diagnósticos presentaron dos fotogramas sobre Vulkan.
En la demostración interactiva se observaron el frente, el lateral derecho,
el techo, la puerta y dos ventanas. Se comprobaron zoom, reinicio y cierre.
Queda por inspeccionar la órbita y las otras fachadas en una sesión manual.

En Sprint 7.2 pasaron formato, check, Clippy estricto, pruebas y build.
Pasaron 77 pruebas base y 81 con `diagnostics`, más tres pruebas de
compilación negativa en cada configuración.
Las cuatro ejecuciones de humo con `PCE_MAX_FRAMES=2`
salieron con código 0, usando la escena clásica, la casa clásica, la casa
pixel art y la casa pixel art con diagnósticos. `cargo run --example house`
y `cargo run --example roof` finalizaron correctamente.

Las pruebas GPU verificaron bloques nearest de 3 × 3, las bandas negras,
la reutilización y sustitución de la textura, la oclusión y los tres tonos
del shader. Se ejecutaron explícitamente mediante
`cargo test --locked --lib gpu_ -- --ignored`.

La demostración interactiva con diagnósticos usó AMD Radeon sobre Vulkan
con una resolución interna de 341 × 256 a escala 3. Se vieron dos fachadas,
el techo, la puerta y las ventanas. Se comprobó acercamiento con la rueda,
retorno con `R`, redimensionamiento al maximizar, reposo con 0 FPS y cierre
correcto con Escape. La inspección interactiva terminó al pulsar Escape.

## Uso de los paquetes de vivienda

Desde la raíz del proyecto, usa directorios nuevos para cada exportación.
El ejemplo toma la semilla 42 y genera un PNG de 256 × 256 a partir de
128 × 128 téxeles lógicos.

```powershell
$base = 'output/assets/house_nueva'
$copia = 'output/assets/house_nueva_reimportada'
cargo run --example export_house -- export $base
Get-ChildItem -LiteralPath $base
cargo run --example export_house -- import "$base/asset.json"
cargo run --example export_house -- render "$base/asset.json" $copia
Get-FileHash -LiteralPath "$base/asset.json","$copia/asset.json","$base/sprite.png","$copia/sprite.png"
```

El ejemplo sin argumentos crea `output/assets/house_0001` si todavía no
existe. La prueba de consola exportó esa carpeta y volvió a generar
`house_0001_reimportada` desde el JSON. Coincidieron ambos pares de SHA-256
en este equipo. La importación informó semilla 42, 13 componentes y
696 triángulos. La prueba de archivo existente devolvió un error.

Las regresiones gráficas en AMD Radeon con Vulkan alcanzaron
`PCE_MAX_FRAMES=2` en escena clásica, casa clásica, casa pixel art y casa
pixel art con diagnósticos. La comparación binaria de PNG se limita a
condiciones controladas de GPU y controlador.

Pasaron formato, check, Clippy estricto y build. La suite ejecutó 87
pruebas base y 91 con diagnósticos, más tres doctests por variante.
Las tres pruebas GPU se ejecutaron por separado con
`cargo test --locked gpu_ -- --ignored`. Los ejemplos `house` y `roof`
también finalizaron correctamente.
