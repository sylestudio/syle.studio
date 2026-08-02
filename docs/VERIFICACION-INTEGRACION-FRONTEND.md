# Acta de revisión para integrar `frontend` a `main`

- Fecha de revisión: 2026-08-02
- Repositorio: `sylestudio/syle.studio`
- Base revisada: `main` en `d13e053b2088a72f9e5a353e7b06aaa921e96b14`
- Aporte revisado: `frontend` en `ece4ae0a38b60279bb3563f07a73c6993324e9e8`
- Autor principal del aporte: Daniel Capistran Morales (`dxnielcapi`)
- Rama de integración: `integration/frontend-production`

## Dictamen

La integración queda técnicamente apta para `main` cuando los mismos gates de
esta acta terminen en verde dentro del pull request. La comprobación posterior
al merge exige, además, que el workflow `deploy` finalice correctamente y que
`https://syle.studio` publique en `syle-revision` el SHA integrado.

El texto de borrador, textos provisionales y correcciones editoriales no forman
parte del bloqueo de este merge, por decisión del propietario. Sí son bloqueo
los defectos funcionales, de seguridad, accesibilidad, procesamiento de medios,
integridad de datos, rendimiento o despliegue.

## Checklist ejecutado

### Alcance e historial

- [x] `main` es ancestro de `frontend`; no hay divergencia ni resolución manual
  de conflictos oculta.
- [x] Se revisaron los 37 commits aportados sobre `main` y se conservó su
  autoría en el historial de la rama.
- [x] Se retiraron `.DS_Store` y el `package-lock.json` mezclado con pnpm.
- [x] `.codegraph/` permanece local y fuera del commit.
- [x] `git diff --check origin/main` no reporta espacios o marcadores inválidos.

### Procesamiento de imágenes en Rust

- [x] El decoder limita cada entrada a 64 MP, 16,384 px por dimensión y 384 MiB
  de asignación para reducir el riesgo de image bombs.
- [x] La orientación EXIF se aplica antes de calcular dimensiones y variantes.
- [x] Las salidas se reconstruyen desde píxeles y no conservan EXIF, XMP, IPTC
  ni ubicación de la imagen de origen.
- [x] AVIF y JPEG se generan en 480, 960, 1,440 y 2,400 px sin escalar hacia
  arriba imágenes pequeñas.
- [x] El trabajo CPU-bound se ejecuta en el pool bloqueante y está limitado a
  dos ingestas simultáneas.
- [x] Los nombres incorporan SHA-256 para identidad de contenido y un UUID de
  propietario para evitar que borrar un registro rompa otro upload idéntico.
- [x] Archivos y filas se publican con staging, renames atómicos y transacciones;
  un fallo parcial revierte el lote de archivos.
- [x] La eliminación confirma primero la verdad en PostgreSQL y después hace
  limpieza best-effort, de modo que un fallo de disco solo puede dejar un
  huérfano y no una referencia activa a un archivo ausente.

### Imágenes estáticas y frontend

- [x] Las 64 imágenes fotográficas originales salieron de `public/` y se
  conservan como fuentes en `web/media-src/`.
- [x] El build produce 486 derivados inmutables AVIF/JPEG y un manifiesto con
  hash de origen, dimensiones, ThumbHash y versión del pipeline.
- [x] La generación es incremental, valida el hash de cada archivo restaurado
  del cache y elimina derivados obsoletos.
- [x] El encoder AVIF usa un número fijo de hilos, por lo que sus hashes son
  reproducibles entre desarrollo, CI y el host de despliegue.
- [x] Todas las páginas de proyecto usan `picture`, `srcset`, `sizes`, ancho,
  alto, lazy loading y placeholder; los héroes usan prioridad alta.
- [x] Ningún original fotográfico pesado se copia al `dist`.
- [x] Los derivados viven en `/project-media/`, fuera del namespace `/media/`
  reservado por nginx/Vite para uploads dinámicos del API.
- [x] Presupuesto automatizado: máximo 1,500,000 bytes por derivado y 90 MiB
  para el conjunto desplegable.
- [x] Se añadió cache de derivados por hash del manifiesto en CI y despliegue.
- [x] El slider respeta `prefers-reduced-motion`, usa botones accesibles y ofrece
  objetivos táctiles de 28 px con foco visible.
- [x] Se corrigieron contraste, landmark principal, metadescripción y marcado
  HTML estricto detectado al migrar a Astro 7.

### Dependencias y automatización

- [x] Astro se actualizó de 5.18.1 a 7.1.6 con Node >=22.12 fijado en proyecto
  y workflows.
- [x] `sharp` 0.35.3 y `svgo` 4.0.2 corrigen los últimos avisos transitivos.
- [x] `pnpm audit --prod`: 0 vulnerabilidades conocidas en web y admin.
- [x] RustSec: se actualizaron `crossbeam-epoch` y `quinn-proto`; el único
  advisory ignorado corresponde a `rsa` dentro del driver MySQL opcional de
  SQLx, que no forma parte de ningún target porque el workspace usa PostgreSQL
  con `default-features = false`.
- [x] CI nuevo valida Rust nativo, CRM WASM, PostgreSQL real, Astro, pruebas y
  presupuestos de imágenes en cada pull request.
- [x] El despliegue rechaza una publicación cuyo HTML no exponga el SHA exacto
  de `GITHUB_SHA` en la metaetiqueta `syle-revision`.

## Evidencia reproducible

| Gate | Comando o entorno | Resultado |
|---|---|---|
| Rust estático | `cargo clippy --workspace --all-targets -- -D warnings` | aprobado |
| Suite Rust + BD | PostgreSQL 17 aislado + `cargo test --workspace --all-targets` | 140 pruebas aprobadas |
| Formato tocado | `rustfmt --check --config skip_children=true ...` | aprobado |
| Astro | `pnpm --dir web check` | 26 archivos, 0 errores, 0 warnings, 0 hints |
| Web unitarias | `pnpm --dir web test` | 7 pruebas aprobadas |
| Web producción | `pnpm --dir web build` | 7 rutas, 64 fuentes verificadas, presupuesto aprobado |
| CRM WASM | `cargo check -p syle-admin --target wasm32-unknown-unknown` | aprobado |
| CRM producción | `trunk build --release --config admin/Trunk.toml` | aprobado |
| Dependencias JS | `pnpm audit --prod` en web y admin | 0 vulnerabilidades conocidas |
| Dependencias Rust | `cargo audit` con excepción documentada de driver no compilado | aprobado |
| Workflows | parseo YAML de `ci.yml`, `deploy.yml` y `site.yml` | aprobado |

Las pruebas de PostgreSQL incluyen orientación y límites del decoder, rollback
de publicación parcial, posiciones concurrentemente seguras y el ciclo de vida
de dos uploads con bytes idénticos: comparten el prefijo de contenido, sus
rutas son distintas y borrar uno no elimina los archivos del otro.

## Medición Lighthouse local

Perfil móvil, Lighthouse 13.4.1, build de producción servido por `astro preview`.
Los valores son evidencia de una corrida controlada y pueden variar ligeramente
según CPU; los límites de archivos sí se ejecutan de forma determinista en CI.

| Ruta | Estado | Rendimiento | Accesibilidad | Buenas prácticas | SEO | LCP | Transferencia |
|---|---|---:|---:|---:|---:|---:|---:|
| `/` | antes | 76 | 96 | — | — | 7.6 s | 1,328 KiB |
| `/` | revisado | 92 | 100 | 100 | 100 | 3.30 s | 465 KiB |
| `/proyectos/insomnio/` | antes | 75 | 95 | — | — | 57.8 s | 12,969 KiB |
| `/proyectos/insomnio/` | revisado | 99 | 100 | 100 | 100 | 2.10 s | 482 KiB |

En ambas rutas revisadas el Total Blocking Time fue 0 ms. El conjunto completo
de derivados ocupa 82.64 MiB en el servidor, pero no se descarga completo: el
navegador selecciona el ancho adecuado y el lazy loading difiere lo no visible.

## Riesgos aceptados y trabajo posterior

- El contenido editorial de borrador queda explícitamente fuera del bloqueo.
- El árbol heredado no cumple un `cargo fmt --all --check` global con el rustfmt
  actual. Los archivos Rust modificados sí están formateados y el diff no añade
  whitespace defectuoso; normalizar todo el repositorio debe ser un cambio
  mecánico separado para no mezclarlo con esta integración.
- La limpieza de archivos después de un `DELETE` es best-effort por diseño. Un
  recolector de huérfanos puede añadirse después; la prioridad actual es no
  dejar filas vivas apuntando a medios inexistentes.
- No se hacen mutaciones de contenido real desde esta revisión. Login, upload y
  publicación autenticados se cubren con pruebas de integración; el smoke test
  de producción se limita a rutas públicas y carga del panel protegido.

## Comprobación del propietario

Después del despliegue, el propietario puede dejar constancia de aceptación sin
repetir la revisión técnica:

- [ ] Confirmé visualmente Inicio y los cinco proyectos en móvil y escritorio.
- [ ] Confirmé que el contenido de borrador pendiente no bloquea esta entrega.
- [ ] Confirmé que `admin.syle.studio` carga el panel y que mi acceso funciona.
- [ ] Acepto integrar el aporte de Daniel y este endurecimiento en `main`.

- Nombre: ______________________________
- Fecha: _______________________________
- Firma o aprobación en PR: ____________

## Evidencia posterior al merge

El pull request conserva revisión, checks y autores. El workflow `deploy` es la
fuente de verdad de la publicación: compila los tres artefactos, verifica el API,
instala el sitio, recarga nginx y compara `syle-revision` con el SHA del job. La
URL del PR, el SHA final y la ejecución se anexan al cierre de la goal.
