# Documento de diseño y planificación técnica
# Generador Procedural de Objetos 3D con Estética Pixel Art

**Proyecto:** Prototipo de generación procedural para un simulador urbano tipo SimCity 4  
**Versión:** 1.0  
**Lenguaje principal:** Rust  
**Enfoque:** Desarrollo de tecnología gráfica propia, sin motores de videojuegos.

---

# 1. Visión general del proyecto

El objetivo es desarrollar un sistema de generación procedural de objetos tridimensionales utilizando Rust.

El sistema permitirá construir objetos mediante geometría matemática, combinando primitivas como cubos, prismas, planos y otras figuras geométricas.

Los objetos generados se visualizarán mediante un sistema de renderizado propio, utilizando una cámara ortográfica con perspectiva diagonal y una estética Pixel Art.

El proyecto forma parte de una iniciativa mayor: desarrollar un videojuego de simulación urbana inspirado en SimCity 4.

Sin embargo, el desarrollo inicial se concentrará exclusivamente en construir y validar el generador procedural.

## 1.1. Concepto principal

No queremos depender de inteligencia artificial para generar imágenes de edificios ni utilizar modelos 3D tradicionales creados manualmente.

Queremos que los objetos sean construidos matemáticamente mediante algoritmos.

Por ejemplo, una casa estará compuesta por diferentes elementos geométricos:

- Paredes.
- Columnas.
- Techos.
- Puertas.
- Ventanas.
- Elementos decorativos.

Cada elemento tendrá geometría, posición, dimensiones y materiales propios.

El resultado será un objeto tridimensional que podrá visualizarse desde diferentes ángulos y distancias.

**Principio fundamental: los objetos deben existir como geometría y datos, no como imágenes estáticas. Las imágenes serán únicamente el resultado del proceso de renderizado.**

# 2. Objetivos del primer prototipo

El primer prototipo tendrá un alcance deliberadamente reducido.

Queremos conseguir lo siguiente:

1. Crear una aplicación de escritorio escrita en Rust.
2. Abrir una ventana gráfica.
3. Construir primitivas geométricas mediante código.
4. Implementar un sistema de caras y vértices.
5. Asignar materiales y colores independientes a las caras.
6. Combinar primitivas para construir objetos.
7. Implementar una cámara ortográfica.
8. Implementar una cámara diagonal similar a SimCity 4.
9. Permitir acercar y alejar la cámara.
10. Implementar un procesamiento visual Pixel Art.
11. Generar proceduralmente una casa sencilla.
12. Utilizar semillas aleatorias para generar variaciones reproducibles.

## 2.1. Qué NO vamos a desarrollar inicialmente

Para evitar ampliar innecesariamente el alcance, quedan fuera del primer prototipo:

- Generación de ciudades completas.
- Simulación de población.
- Economía urbana.
- Vehículos.
- Personajes.
- Sistema de incendios.
- Destrucción de edificios.
- Clima.
- Mapas y terrenos complejos.
- Inteligencia artificial generativa.
- Editor avanzado de edificios.

Estas funcionalidades podrán desarrollarse posteriormente utilizando la arquitectura creada.

# 3. Lenguaje y tecnologías

El desarrollo se realizará principalmente en Rust.

No utilizaremos Godot, Unity, Unreal Engine ni otros motores de videojuegos.

Podremos utilizar bibliotecas de bajo nivel para acceder a funcionalidades del sistema operativo y del hardware gráfico.

## 3.1. Tecnologías seleccionadas

| Tecnología | Responsabilidad |
|---|---|
| Rust | Lenguaje principal del proyecto. |
| Cargo | Gestión del proyecto y dependencias. |
| wgpu | Acceso a GPU y operaciones de renderizado. |
| winit | Creación de ventanas y gestión de eventos. |
| glam | Matemáticas 3D, vectores y matrices. |
| bytemuck | Conversión segura de datos para buffers gráficos. |
| image | Procesamiento y exportación de imágenes. |
| serde | Serialización de configuraciones y objetos. |
| rand / rand_chacha | Generación aleatoria reproducible. |

Las versiones concretas de las dependencias deberán seleccionarse al iniciar el proyecto y quedar fijadas en Cargo.lock.

## 3.2. Responsabilidad de cada tecnología

**Rust**

Será responsable de toda la lógica del proyecto:

- Generación geométrica.
- Construcción de objetos.
- Gestión de materiales.
- Transformaciones espaciales.
- Cámara.
- Procesamiento de escenas.
- Algoritmos de generación procedural.

**wgpu**

Será la interfaz gráfica de bajo nivel que permitirá utilizar la GPU.

No será nuestro motor gráfico. Nosotros desarrollaremos la lógica de renderizado y los shaders.

**winit**

Gestionará la ventana de la aplicación, los eventos de teclado y el ratón.

**glam**

Permitirá realizar operaciones matemáticas como:

- Vectores tridimensionales.
- Matrices de transformación.
- Rotaciones.
- Escalado.
- Proyecciones.
- Posicionamiento de la cámara.

**image**

Se utilizará para procesar imágenes y, posteriormente, exportar los resultados como PNG.

**serde**

Permitirá guardar las configuraciones de los edificios en archivos JSON.

**rand_chacha**

Permitirá generar variaciones utilizando una semilla determinada. Una misma semilla y una misma configuración deberán producir resultados reproducibles.

# 4. Arquitectura conceptual

El sistema estará dividido en módulos independientes.

La separación entre generación procedural y renderizado es fundamental.

## 4.1. Generador procedural

Su responsabilidad será construir los objetos.

No deberá depender directamente de la ventana ni de la GPU.

Recibirá parámetros y producirá estructuras geométricas.

Ejemplo:

Una solicitud de generación de una casa podrá contener:

- Ancho.
- Altura.
- Profundidad.
- Número de pisos.
- Tipo de techo.
- Color de paredes.
- Tipo de ventanas.
- Tipo de puertas.
- Semilla aleatoria.

El resultado será una estructura de datos que represente la casa.

## 4.2. Sistema geométrico

Será el encargado de crear las primitivas fundamentales.

Inicialmente implementaremos:

1. Cubo.
2. Prisma rectangular.
3. Plano.
4. Superficie cuadrilateral.
5. Triángulo.

Cada objeto tendrá vértices, caras, normales y materiales.

Aunque inicialmente trabajaremos con volúmenes de seis caras, la arquitectura no deberá limitarse a esa cantidad.

Utilizaremos una representación de mallas que permita incorporar geometrías más complejas en el futuro.

## 4.3. Sistema de materiales

Cada cara podrá tener propiedades independientes.

Por ejemplo:

- Color.
- Identificador de textura.
- Material.
- Propiedades de iluminación.
- Configuración de contorno.

La idea es poder construir una casa donde la fachada frontal tenga un color diferente de las paredes laterales y el techo.

## 4.4. Sistema de renderizado

Recibirá la geometría generada y se encargará de representarla.

Inicialmente deberá soportar:

- Proyección ortográfica.
- Cámara diagonal.
- Profundidad.
- Eliminación de caras no visibles cuando corresponda.
- Orden correcto de dibujo.
- Materiales.
- Colores.
- Iluminación básica.

Posteriormente incorporaremos sombras, efectos, texturas y optimización.

## 4.5. Sistema Pixel Art

No queremos simplemente aplicar un filtro de pixelado a cualquier imagen.

Queremos controlar el estilo gráfico desde el proceso de renderizado.

Inicialmente utilizaremos:

- Resolución de renderizado reducida.
- Escalado mediante Nearest Neighbor.
- Paletas de colores limitadas.
- Contornos definidos.
- Iluminación por niveles.
- Sombras discretas.
- Ausencia de suavizado que destruya los bordes de los píxeles.

La resolución inicial será configurable.

# 5. Concepto de construcción de objetos

Los edificios se construirán mediante la combinación de diferentes primitivas geométricas.

Por ejemplo, una casa sencilla estará formada por:

| Componente | Representación |
|---|---|
| Estructura principal | Prisma rectangular |
| Techo | Prisma o conjunto de superficies |
| Puerta | Panel geométrico |
| Ventanas | Paneles con materiales propios |
| Columnas | Prismas estrechos |
| Decoración | Geometría adicional |

Cada componente tendrá una posición relativa al objeto principal.

Esto permitirá generar diferentes casas reutilizando las mismas primitivas.

## 5.1. Ejemplo de configuración

Una casa podrá generarse utilizando una configuración como esta:

```json
{
  "type": "house",
  "seed": 12345,
  "dimensions": {
    "width": 8,
    "height": 5,
    "depth": 7
  },
  "floors": 1,
  "roof": {
    "type": "flat"
  },
  "facade": {
    "color": "#AAB5C4"
  },
  "windows": {
    "count": 4
  },
  "doors": {
    "count": 1
  }
}
```

Esta configuración es ilustrativa. El esquema definitivo se establecerá durante la implementación.

# 6. Estructura inicial del proyecto

Se propone comenzar con la siguiente organización:

```text
procedural_city/
│
├── Cargo.toml
├── Cargo.lock
│
├── assets/
│   ├── textures/
│   └── palettes/
│
├── src/
│   │
│   ├── main.rs
│   │
│   ├── core/
│   │   ├── mod.rs
│   │   ├── math.rs
│   │   └── config.rs
│   │
│   ├── geometry/
│   │   ├── mod.rs
│   │   ├── vertex.rs
│   │   ├── face.rs
│   │   ├── mesh.rs
│   │   └── primitives/
│   │       ├── mod.rs
│   │       ├── cube.rs
│   │       ├── prism.rs
│   │       └── plane.rs
│   │
│   ├── procedural/
│   │   ├── mod.rs
│   │   ├── generator.rs
│   │   ├── buildings/
│   │   │   ├── mod.rs
│   │   │   └── house.rs
│   │   └── random.rs
│   │
│   ├── materials/
│   │   ├── mod.rs
│   │   └── material.rs
│   │
│   ├── camera/
│   │   ├── mod.rs
│   │   └── orthographic.rs
│   │
│   ├── rendering/
│   │   ├── mod.rs
│   │   ├── renderer.rs
│   │   ├── pipeline.rs
│   │   ├── shaders/
│   │   └── pixel_art.rs
│   │
│   └── application/
│       ├── mod.rs
│       ├── window.rs
│       └── input.rs
│
└── tests/
    ├── geometry_tests.rs
    └── procedural_tests.rs
```

Esta estructura es una propuesta inicial. No es necesario implementar todos los archivos desde el primer día.

Los módulos deberán crearse progresivamente, a medida que sean necesarios.

# 7. Plan de desarrollo Scrum

El desarrollo se organizará en sprints pequeños.

Cada sprint tendrá un objetivo, tareas y criterios de aceptación.

No avanzaremos a la siguiente etapa hasta comprobar que la anterior funciona.

## Sprint 0. Preparación del entorno

**Objetivo:** preparar el proyecto y comprobar que Rust puede abrir una ventana gráfica.

Tareas:

- Instalar Rust y Cargo.
- Instalar las herramientas necesarias de desarrollo.
- Crear el proyecto.
- Configurar las dependencias.
- Integrar winit.
- Inicializar wgpu.
- Crear una ventana.
- Configurar el bucle principal de eventos.

Entregable:

Una aplicación Rust que abre una ventana y puede cerrarse correctamente.

Criterios de aceptación:

- El proyecto compila.
- La ventana aparece.
- No hay errores críticos.
- La aplicación responde correctamente a los eventos de cierre.

## Sprint 1. Sistema matemático y geométrico

**Objetivo:** construir las primitivas geométricas básicas.

Tareas:

- Definir la estructura Vertex.
- Definir la estructura Face.
- Definir la estructura Mesh.
- Implementar un cubo.
- Generar sus ocho vértices.
- Generar sus caras.
- Implementar un prisma rectangular.
- Asignar un material independiente a cada cara.
- Crear pruebas unitarias para la geometría.

Entregable:

Un sistema capaz de generar cubos y prismas mediante código.

Criterios de aceptación:

- El cubo tiene ocho vértices.
- Su geometría está correctamente definida.
- Sus seis caras están identificadas.
- Los materiales se pueden modificar independientemente.
- El prisma permite modificar ancho, altura y profundidad.
- Las pruebas geométricas pasan correctamente.

## Sprint 2. Primer renderizado 3D

**Objetivo:** visualizar las primitivas generadas.

Tareas:

- Crear el pipeline gráfico.
- Crear los buffers de vértices.
- Crear los buffers de índices.
- Implementar los shaders iniciales.
- Enviar la geometría a la GPU.
- Dibujar un cubo.
- Implementar profundidad.
- Implementar colores por cara.

Entregable:

Un cubo visible en la ventana.

Criterios de aceptación:

- El cubo se representa correctamente.
- Cada cara tiene su propio color.
- Las caras se muestran en el orden correcto.
- Los objetos se pueden visualizar sin deformaciones.

## Sprint 3. Cámara y perspectiva

**Objetivo:** implementar la cámara diagonal característica del simulador.

Tareas:

- Implementar cámara ortográfica.
- Definir la posición inicial de la cámara.
- Implementar la matriz de vista.
- Implementar la matriz de proyección.
- Implementar zoom.
- Implementar rotación.
- Probar diferentes posiciones de cámara.

Entregable:

Una cámara capaz de observar el cubo desde diferentes ángulos.

Criterios de aceptación:

- La cámara utiliza proyección ortográfica.
- El zoom funciona.
- La rotación funciona.
- El cubo conserva sus proporciones.
- La perspectiva diagonal es consistente.

## Sprint 4. Sistema de construcción modular

**Objetivo:** combinar primitivas para crear objetos más complejos.

Tareas:

- Implementar transformaciones de posición.
- Implementar rotación y escalado.
- Combinar diferentes mallas.
- Crear un sistema de componentes.
- Implementar materiales reutilizables.
- Crear una estructura de objeto compuesto.

Entregable:

Un objeto compuesto formado por diferentes primitivas.

Criterios de aceptación:

- Se pueden combinar varios prismas.
- Cada componente conserva su posición relativa.
- Los materiales son independientes.
- Los objetos se pueden modificar sin reconstruir manualmente toda la escena.

## Sprint 5. Generador procedural de casas

**Objetivo:** construir una casa sencilla mediante parámetros.

Tareas:

- Crear el módulo HouseGenerator.
- Definir las dimensiones de la casa.
- Generar las paredes.
- Generar el techo.
- Generar una puerta.
- Generar ventanas.
- Aplicar materiales.
- Implementar variaciones controladas.
- Integrar semillas aleatorias.

Entregable:

Una casa generada automáticamente.

Criterios de aceptación:

- La casa se genera sin modelado manual.
- Se pueden modificar sus dimensiones.
- Se pueden modificar sus colores.
- Se pueden modificar sus ventanas.
- Se puede cambiar su techo.
- La misma semilla produce el mismo resultado.
- Una semilla diferente permite generar variaciones.

## Sprint 6. Apariencia Pixel Art

**Objetivo:** conseguir que la casa tenga la estética gráfica deseada.

Tareas:

- Implementar renderizado a baja resolución.
- Implementar escalado Nearest Neighbor.
- Crear una paleta de colores.
- Implementar iluminación por niveles.
- Implementar sombras básicas.
- Definir contornos.
- Ajustar la resolución y el tamaño visual de los píxeles.

Entregable:

Una casa procedural con apariencia Pixel Art.

Criterios de aceptación:

- Los píxeles tienen bordes definidos.
- No aparecen desenfoques indeseados.
- La paleta se aplica correctamente.
- Los contornos son consistentes.
- La cámara conserva la estética visual al acercarse y alejarse.

## Sprint 7. Consolidación y exportación

**Objetivo:** estabilizar el prototipo y preparar su evolución.

Tareas:

- Guardar configuraciones en JSON.
- Cargar configuraciones.
- Exportar imágenes PNG.
- Probar distintas semillas.
- Probar distintas dimensiones.
- Corregir errores geométricos.
- Documentar los módulos.
- Medir el rendimiento inicial.

Entregable:

Un generador procedural funcional capaz de producir diferentes casas y visualizar sus resultados.

Criterios de aceptación:

- Se pueden guardar y recuperar configuraciones.
- Se pueden generar múltiples casas.
- Los resultados son reproducibles.
- La exportación funciona.
- La aplicación no presenta errores críticos.

# 8. Resultado esperado del MVP

Al finalizar los sprints, tendremos una aplicación de escritorio escrita en Rust.

La aplicación deberá permitir:

1. Abrir una ventana.
2. Generar una casa mediante parámetros.
3. Modificar sus dimensiones.
4. Cambiar sus materiales.
5. Añadir o quitar ventanas.
6. Modificar el techo.
7. Generar variaciones con semillas.
8. Observar la casa desde una cámara diagonal.
9. Acercar y alejar la cámara.
10. Visualizarla con estética Pixel Art.
11. Exportar el resultado.

El MVP no necesita generar una ciudad completa.

Necesitamos demostrar que nuestra tecnología funciona y que puede convertirse en la base de un motor de generación de objetos urbanos.

# 9. Reglas de desarrollo para trabajar con Gemini

Gemini será utilizado como asistente de programación.

Para mantener la coherencia técnica del proyecto, deberá respetar las siguientes reglas:

1. Todo el código principal debe escribirse en Rust.
2. No introducir Godot, Unity, Unreal ni otros motores gráficos.
3. No sustituir el renderizador propio por un motor externo.
4. Utilizar las bibliotecas seleccionadas cuando corresponda.
5. Mantener separadas la geometría, la generación procedural y el renderizado.
6. No crear funcionalidades fuera del sprint actual sin autorización.
7. No modificar la arquitectura general sin explicar primero el motivo.
8. No generar archivos innecesarios.
9. No duplicar funcionalidades existentes.
10. Documentar las estructuras y funciones importantes.
11. Implementar pruebas cuando corresponda.
12. Verificar que el proyecto compile después de cada modificación.
13. Explicar qué archivos se modificaron.
14. No afirmar que algo funciona si no se ha probado.
15. Evitar soluciones temporales que comprometan la arquitectura futura.

## 9.1. Forma de trabajo

Cada tarea se desarrollará siguiendo este procedimiento:

**Paso 1. Análisis**

Gemini deberá revisar el estado actual del proyecto y explicar qué se necesita implementar.

**Paso 2. Diseño**

Antes de escribir código, deberá explicar las estructuras, funciones y archivos que pretende modificar.

**Paso 3. Implementación**

Implementará únicamente la tarea solicitada.

**Paso 4. Validación**

Deberá ejecutar o indicar los comandos necesarios para compilar y probar.

**Paso 5. Documentación**

Deberá indicar qué se construyó, qué archivos cambiaron y qué queda pendiente.

**Paso 6. Confirmación**

No avanzará automáticamente a funcionalidades del siguiente sprint.

# 10. Prompt inicial para Gemini

El siguiente prompt será utilizado para iniciar el proyecto:

---

Actúa como arquitecto de software e ingeniero especializado en Rust, programación gráfica, geometría computacional y generación procedural.

Vamos a desarrollar un proyecto llamado Procedural City Engine.

El objetivo final es construir tecnología propia para un simulador urbano inspirado en SimCity 4.

No utilizaremos motores gráficos como Godot, Unity o Unreal Engine.

El lenguaje principal será Rust.

Utilizaremos wgpu como interfaz de renderizado de bajo nivel, winit para ventanas y eventos, glam para matemáticas 3D y otras bibliotecas auxiliares cuando sean necesarias.

El sistema tendrá dos componentes fundamentales:

1. Un generador procedural que construirá objetos mediante geometría matemática.
2. Un renderizador propio que visualizará esa geometría utilizando una cámara ortográfica y una estética Pixel Art.

La primera meta es construir una casa sencilla mediante cubos, prismas y superficies geométricas.

La arquitectura debe ser modular y permitir ampliar el sistema en el futuro para generar edificios, vehículos, árboles y otros elementos urbanos.

La generación procedural debe ser independiente del renderizado.

Debemos utilizar semillas aleatorias reproducibles.

No queremos generar imágenes mediante IA ni utilizar imágenes estáticas como representación principal de los objetos.

Quiero que sigas un desarrollo incremental basado en los sprints del documento de planificación.

**Instrucciones obligatorias:**

- No implementes todo el proyecto de una sola vez.
- No avances al siguiente sprint sin autorización.
- Primero analiza los requisitos de la tarea actual.
- Explica qué vas a construir.
- Indica qué archivos serán creados o modificados.
- Implementa únicamente lo necesario.
- Comprueba la compilación.
- Explica cómo probar el resultado.
- No cambies las tecnologías ni la arquitectura sin justificarlo.

Comienza exclusivamente con el Sprint 0: preparación del entorno, creación del proyecto Rust, ventana con winit e inicialización básica de wgpu.

No implementes todavía geometría, edificios ni generación procedural.

Primero quiero comprobar que la base gráfica funciona correctamente.

---

# 11. Criterio de finalización del proyecto inicial

El primer objetivo se considerará cumplido cuando podamos abrir nuestra aplicación, generar una casa automáticamente y observarla desde una cámara diagonal.

La casa deberá estar construida mediante geometría generada por código y no mediante una imagen precargada.

Este resultado demostrará que tenemos las bases necesarias para continuar desarrollando nuestro propio sistema de generación procedural.

---

# 12. Visión de evolución futura

Una vez completado el MVP, podremos ampliar el proyecto mediante nuevos módulos:

| Etapa | Funcionalidad |
|---|---|
| 1 | Generador de edificios residenciales. |
| 2 | Generador de edificios comerciales e industriales. |
| 3 | Generador de árboles y vegetación. |
| 4 | Generador de vehículos y mobiliario urbano. |
| 5 | Generador de terrenos y calles. |
| 6 | Generación procedural de barrios. |
| 7 | Sistema de incendios y efectos ambientales. |
| 8 | Sistema de daños y destrucción. |
| 9 | Simulación de población y economía. |
| 10 | Construcción del simulador urbano completo. |

**Objetivo a largo plazo:** disponer de una tecnología propia, escrita en Rust, capaz de generar, representar y simular los elementos de una ciudad completa, manteniendo consistencia geométrica y artística.

---

**Fin del documento de planificación técnica.**