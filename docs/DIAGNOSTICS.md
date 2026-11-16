# Diagnóstico opcional

## Propósito

La característica Cargo `diagnostics` observa las presentaciones exitosas de
fotogramas durante el desarrollo. No depende de la geometría ni del renderizador
interno, y no controla la cámara, los recursos GPU ni las solicitudes de dibujo.

## Uso

Ejecutar el motor sin diagnóstico:

```powershell
cargo run
```

Ejecutarlo con diagnóstico:

```powershell
cargo run --features diagnostics
```

Con la característica activada, el título se actualiza aproximadamente una vez
por segundo. La actualización del título no llama a `request_redraw()` ni ejecuta
el renderizador.

## Métricas

- `FPS`: presentaciones exitosas observadas durante el último intervalo cercano
  a un segundo. No cuenta eventos de redibujado fallidos.
- `Frame`: duración en CPU de la llamada de renderizado que terminó con
  presentación exitosa. No es una medición del tiempo de ejecución en GPU.
- `Total`: número acumulado de presentaciones exitosas desde el inicio.
- `ACTIVE`: hubo una presentación dentro del último intervalo de actividad.
- `IDLE`: no hubo presentaciones recientes; en ese caso `FPS` es cero y el tiempo
  de fotograma se muestra como `--`.

`Duration` representa duraciones válidas por construcción. Una duración cero se
acepta y se presenta como un dato disponible; la ausencia de un fotograma se
representa explícitamente con `--`.

## Temporización y eliminación

En reposo, `diagnostics` usa `WaitUntil` solo para el próximo refresco del título.
Durante movimiento continuo de cámara se combina ese vencimiento con el de 16 ms
de WASD y se espera siempre el más próximo. No se crean hilos ni bucles ocupados.

Para retirarlo completamente, elimine la característica `diagnostics` de
`Cargo.toml`, el módulo `src/diagnostics.rs`, las referencias condicionales en
`src/lib.rs` y `src/app.rs`, y este documento. La configuración sin la
característica ya compila sin referencias al módulo.
