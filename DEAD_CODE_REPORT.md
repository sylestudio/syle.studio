# Reporte de código muerto

Última actualización: 2026-08-17
Estado del build/tests base: **ok**

- `npm test` (vitest) → 7/7 pasan (`src/lib/roman.test.ts`, `src/lib/images.test.ts`)
- `npm run build` (astro) → ok, 7 páginas generadas
- Working tree al inicio: `web/src/pages/index.astro` modificado sin commitear (no tocado por esta auditoría)

## Alcance de esta sesión

**Solo `web/`** (app Astro SSG). Decidido explícitamente con el propietario.

El workspace Rust (`api`, `core`, `types`, `render`, `imageedit`, `admin`) **queda fuera y sin analizar**: en esta máquina no hay toolchain de Rust (no existe `~/.cargo`, `~/.rustup/toolchains` está vacío, no hay `cargo`/`rustc`/`rustup` en PATH). Sin eso no hay `cargo check`, `cargo test`, `clippy`, warnings de `dead_code` ni `cargo machete`, y tampoco forma de validar un borrado. Retomar cuando haya toolchain — ver "Siguiente paso".

**Trampa conocida para la próxima iteración en Rust:** `admin/` está excluido de `default-members` en `Cargo.toml`. Cualquier herramienta que corra sobre el workspace por defecto **no verá los usos que `admin` hace de `types/`**, y reportará como muerto código que sí se usa. Hay que forzar `--workspace` y `--target wasm32-unknown-unknown` para `admin`.

## Contexto

- **Tipo de proyecto:** aplicación (no librería). Todo lo no alcanzable desde las páginas es candidato; no hay API pública que preservar.
- **Puntos de entrada:** `web/src/pages/**` (7 rutas) + scripts `dev`/`build`/`preview`/`test` de `package.json`.
- **Rutas dinámicas:** `blog/[slug]` y `galleries/[slug]` generan 0 páginas en local porque la API no está levantada. **No son código muerto** — su `getStaticPaths` consume la API en build time (ver `.github/workflows/site.yml`, que espera a que la API responda 200 antes de construir).
- **Gestor de paquetes en CI:** pnpm (`pnpm --dir web install --frozen-lockfile`) en ambos workflows.

## Resumen

- **Confirmados: 3** | Probables: 8 | Riesgosos: 3 | Eliminados: 0

## Hallazgos

### [CONFIRMADO] `web/package.json:17` — dependencia `marked`

- **Tipo:** dependencia de producción no utilizada
- **Evidencia:**
  - `knip` la reporta como única "unused dependency"; `depcheck` también.
  - `grep -rnw "marked"` en **todo el repo** (excluyendo `node_modules`, `dist`, lockfiles) → **1 sola ocurrencia**: su propia declaración en `package.json:17`.
  - Historia: introducida en `71bfc2b feat(web): add astro public portfolio and blog site`; su último uso se eliminó en `aee5d74 feat(web): render blog posts from API block-rendered HTML` (2026-05-28), que borra `-import { marked } from "marked";` y `-const html = marked.parse(post.body_md);` y los sustituye por `set:html={post.body_html}`. La declaración quedó huérfana.
  - Comprobado que no se usa desde configs (`astro.config.mjs`), scripts de `package.json`, workflows de CI ni `deploy/`.
  - El markdown ya lo renderiza el crate Rust `syle-render` en el servidor — ver comentario en `web/src/pages/blog/[slug].astro:27`.
- **Riesgo de borrado:** bajo
- **Cascada:** ninguna. Al eliminarla hay que regenerar lockfiles con **pnpm** (el que usa CI), no a mano.
- **Estado:** [ ] pendiente

### [CONFIRMADO] `web/src/components/Footer.astro:2` — const `year`

- **Tipo:** variable declarada y nunca usada
- **Evidencia:** `const year = "2025";` es la **única** ocurrencia de `year` en el archivo (`grep -n "year" src/components/Footer.astro` → 1 línea). El markup del footer no interpola `{year}` en ningún sitio. Astro no inyecta variables de frontmatter por convención (solo `Props`/`Astro.props`), así que no hay uso implícito. Introducida en `50a2a5c feat(web): editorial-restraint redesign`; el footer se rediseñó después sin la línea de copyright.
- **Riesgo de borrado:** bajo
- **Estado:** [ ] pendiente

### [CONFIRMADO] 6 × `.DS_Store` versionados — se publican en producción

- **Tipo:** basura de macOS trackeada en git y desplegada al servidor
- **Archivos:** `web/.DS_Store`, `web/public/.DS_Store`, `web/public/img/.DS_Store`, `web/public/img/Dango/.DS_Store`, `web/public/img/Insomnio/.DS_Store`, `web/public/img/Simbolo/.DS_Store` (~41 KB en total)
- **Evidencia:**
  - `git ls-files | grep -i DS_Store` → los 6 están **trackeados**.
  - Cero referencias en `src/`, configs o workflows.
  - Astro copia `public/` verbatim: `find dist -name ".DS_Store"` → **5 de ellos aparecen en `web/dist/`**, y `deploy.yml`/`site.yml` hacen `rsync -a --delete web/dist/ /opt/syle/web/`. Es decir, **hoy se sirven públicamente** en syle.studio.
  - `.gitignore` no contempla `.DS_Store` (por eso se colaron).
- **Riesgo de borrado:** nulo. Además de peso muerto, un `.DS_Store` servido expone nombres de archivos del directorio.
- **Nota:** conviene añadir `.DS_Store` a `.gitignore` en el mismo lote para que no reaparezcan. Es una línea de config, no un refactor — confirmar si se quiere incluir.
- **Estado:** [ ] pendiente

---

### [PROBABLE] 6 imágenes en `web/public/` sin ninguna referencia (~1.6 MB)

| Archivo | Peso |
|---|---|
| `web/public/img/Dango/Manposteria/Man_1.jpg` | 487 KB |
| `web/public/img/Dango/Manposteria/Man_9.jpg` | 332 KB |
| `web/public/img/Dango/Manposteria/Man_7.jpg` | 249 KB |
| `web/public/img/Obsesion/obs_3.jpg` | 188 KB |
| `web/public/img/Dango/Manposteria/Man_10.jpg` | 169 KB |
| `web/public/img/Dango/Manposteria/Man_3.jpg` | 129 KB |
| `web/public/img/Logos/LogoRitual.svg` | 92 KB |

- **Tipo:** assets estáticos huérfanos (se copian a `dist/` y se despliegan)
- **Evidencia:**
  - Diff exacto por ruta (no por substring) entre los 71 paths de asset referenciados en `src/**` y los archivos reales de `public/**`.
  - `grep -rI -F` de cada nombre sobre **todo el repo** (incluye `.astro`, `.ts`, workflows, `deploy/`, `README`, confs de nginx) → **0 ocurrencias** para cada uno.
  - Descartado uso dinámico: revisadas todas las referencias con template literal (`src={...}`, `` `/img...` ``, `${}`) y no hay `import.meta.glob` ni construcción de rutas por índice. Las galerías se alimentan de arrays literales en el frontmatter (`slides`, `clients` en `index.astro`; `works` en `syle.astro`) y de `<img src="...">` literales en las páginas de proyecto — todos capturados por el grep.
  - `Man_2,4,5,6,8,11,12,13,14` **sí** se referencian en `dango.astro:47-71`; los 5 faltantes de la serie no.
- **Por qué PROBABLE y no CONFIRMADO:** son contenido editorial de un portafolio, no código. Técnicamente el borrado es seguro (cero referencias, build no los toca), pero la decisión de curaduría es del propietario: pueden ser descartes de una selección en curso.
- **Riesgo de borrado:** bajo técnicamente, **requiere tu aprobación** por ser contenido.
- **Estado:** [ ] pendiente

### [PROBABLE] `web/package-lock.json` — lockfile redundante (218 KB)

- **Tipo:** artefacto de un segundo gestor de paquetes
- **Evidencia:** conviven `web/package-lock.json` y `web/pnpm-lock.yaml`, ambos trackeados. Los **dos** workflows (`deploy.yml`, `site.yml`) usan exclusivamente `pnpm --dir web install --frozen-lockfile`; `npm` no aparece en CI para `web/` salvo para instalar pnpm globalmente. Existe además `web/pnpm-workspace.yaml`. Tener dos lockfiles permite que las resoluciones diverjan silenciosamente entre local y CI.
- **Por qué PROBABLE:** puede que uses `npm` en local a propósito (yo mismo corrí `npm test`/`npm run build` con el `node_modules` ya instalado). Es una decisión de flujo de trabajo, no código muerto puro. **No elijo por ti.**
- **Riesgo de borrado:** medio — cambia cómo instalas en local.
- **Estado:** [ ] pendiente

---

### [RIESGOSO] `web/public/img/Syle_Fuente_Blanco (1).svg` y `Syle_Logo_Blanco (1).svg`

- **Tipo:** assets de marca sin referencias (3.9 KB + 2.6 KB)
- **Evidencia:** 0 ocurrencias en todo el repo. Sus contrapartes en negro (`Syle_Fuente_Negro (1).svg`, `Syle_Logo_Negro (1).svg`) sí se usan en `Footer.astro:9,28`.
- **Por qué RIESGOSO — no tocar:** `index.astro:35` documenta explícitamente el contrato `inv → true = fondo oscuro (usa variante blanca del logo)`, y el array `clients` ya tiene entradas con `inv: true` (líneas 39, 42) que hoy usan el logo negro. Son la mitad de una feature a medio cablear, no restos. Borrarlos destruiría los originales del diseñador.
- **Estado:** documentado, **no se elimina**

### [RIESGOSO] `web/src/pages/proyectos/syle.astro` — página autodescrita como "de prueba"

- **Evidencia:** `syle.astro:7` dice literalmente `// SYLE STUDIO PAGINA DE PRUEBA PARA MENU DE PROYECTOS`. Pero **está viva**: se construye en `/proyectos/syle/` y `index.astro:27` la enlaza desde el último slide del hero (`url: "/proyectos/syle"`).
- **Por qué RIESGOSO — no tocar:** es alcanzable y enlazada desde el home. No es código muerto; es un experimento en producción. Solo lo señalo por si querías que no estuviera publicada.
- **Estado:** documentado, **no se elimina**

### [RIESGOSO] `web/src/pages/proyectos/dango.astro:47` — referencia rota (bug, no código muerto)

- **Evidencia:** `<img src="/img/Dango/Manposteria/TuVertical.jpg">` pero `web/public/img/Dango/Manposteria/TuVertical.jpg` **no existe** (única ocurrencia del nombre en el repo). La galería de mampostería sirve hoy un 404 en ese slot.
- **Por qué aquí:** es el problema inverso al código muerto y arreglarlo sería un cambio funcional, fuera del alcance de "solo eliminar". Lo reporto para que decidas.
- **Estado:** documentado, **no se toca**

---

## Dependencias (Fase 2.5)

| Paquete | Clasificación | Evidencia | Acción | Estado |
|---|---|---|---|---|
| `marked` | **No utilizada** | 0 imports en todo el repo; ausente de configs, scripts y CI; su uso murió en `aee5d74` al pasar a `body_html` del servidor | eliminar + regenerar `pnpm-lock.yaml` | [ ] |
| `@fontsource-variable/space-grotesk` | Utilizada | `Base.astro:2` | ninguna | — |
| `@fontsource/space-mono` | Utilizada | `Base.astro:3-4` (`/400.css`, `/700.css`) | ninguna | — |
| `lenis` | Utilizada | `Base.astro:5` (CSS) y `Base.astro:128` (`import Lenis from "lenis"`) | ninguna | — |
| `thumbhash` | Utilizada | `src/lib/images.ts:1` | ninguna | — |
| `astro` | Utilizada | framework; entrypoint de build | ninguna | — |
| `typescript` (dev) | Utilizada | `tsconfig.json` extiende `astro/tsconfigs/strict`; requerida por `astro check`; además llega transitivamente vía `astro → tsconfck`/`zod-to-ts` | ninguna | — |
| `vitest` (dev) | Utilizada | script `test` | ninguna | — |

- **Fantasma:** ninguna detectada. Todo import de `src/**` resuelve a un paquete declarado.
- **Mal ubicadas:** ninguna. Las 6 `dependencies` son todas de runtime/estilos del sitio; las 2 `devDependencies` son tooling.
- **Redundantes:** ninguna entre paquetes. (El caso `package-lock.json` vs `pnpm-lock.yaml` es de lockfiles, no de paquetes — ver arriba.)
- **Reemplazables:** ninguna trivial.
- **Sin mantenimiento:** ninguna señalada. No se revisaron versiones ni vulnerabilidades: fuera de alcance por instrucción explícita (no modernizar).
- **Cascada:** ninguna dependencia queda huérfana al borrar los CONFIRMADOS. `marked` ya está huérfana por sí sola.

---

## Descartados

Candidatos que las herramientas marcaron pero **sí se usan**. No volver a analizarlos.

| Candidato | Marcado por | Por qué NO es código muerto |
|---|---|---|
| `@fontsource-variable/space-grotesk`, `@fontsource/space-mono`, `lenis` | depcheck | **depcheck no parsea `.astro`**. Los tres se importan en `src/layouts/Base.astro`. Falso positivo sistemático. |
| `typescript` | depcheck | Consumida vía `tsconfig.json` (`extends: astro/tsconfigs/strict`) y por `astro check`; depcheck no lee tsconfig. |
| `listGalleries`, `getGallery`, `listPosts`, `getPost` | ts-prune | **ts-prune no parsea `.astro`**. Usadas en `index.astro:5`, `galleries/[slug].astro:4`, `blog/[slug].astro:3`, `blog/index.astro:3`. |
| Tipos `Photo`, `Gallery`, `GalleryDetail`, `BlogPost`, `ImageVariant` (`lib/api.ts`) | ts-prune | Usados como genéricos de retorno dentro del módulo, en `lib/images.ts:2` y en `components/Photo.astro:2`. `ts-prune` los marca "(used in module)". |
| `thumbDataUrl` (`lib/images.ts:14`) | ts-prune | Usada en `components/Photo.astro:3,14`. |
| `hexToBytes` (`lib/images.ts:5`) | — | Usada internamente por `thumbDataUrl` **y** cubierta por `src/lib/images.test.ts:7,10`. |
| `Sources` (`lib/images.ts:23`) | ts-prune | Tipo de retorno de `buildSources`; debe seguir exportado para que los consumidores lo nombren. |
| `toRoman` (`lib/roman.ts`) | — | Usada en `galleries/[slug].astro:5,38`. Con test propio. |
| `interface Props` × 6 (`Base`, `ProyectoBase`, `Photo`, `Mark`, `Wordmark`) | scan propio de símbolos sin uso | **Convención del framework**: Astro tipa `Astro.props` a partir de `interface Props` por nombre. Uso implícito, nunca referenciado explícitamente. |
| `Mark.astro` | — | Usado en `index.astro:4,51`. |
| `Wordmark.astro` | — | Usado en `Nav.astro:7,13`. |
| `Photo.astro` | — | Usado en `index.astro:3,108` y `galleries/[slug].astro:3,71,93`. |
| `.disp`, `.lbl`, `.ln` (`Base.astro`) | scan propio de CSS | Definidas en `<style is:global>` y usadas desde **otras** páginas (p. ej. `Footer.astro:13`, `index.astro:52-53`). El scan solo miraba el archivo de definición. |
| `default` de `astro.config.mjs` | ts-prune | Es el entrypoint de configuración de Astro. |
| Rutas `blog/[slug]`, `galleries/[slug]` | build local (0 páginas) | Generan 0 páginas **solo porque la API no corre en local**. En CI se espera a que la API dé 200 antes de construir (`site.yml`). |
| `web/public/img/Dango/Manposteria/Man_2,4,5,6,8,11,12,13,14.jpg` | primer scan (buggy) | Un scan intermedio por substring dio falsos positivos/negativos en la serie `Man_*`. El diff exacto por ruta confirma que estas 9 **sí** se referencian en `dango.astro:47-71`. |
| `/img/logos/cliente.svg` | scan de refs rotas | Aparece solo dentro de un **comentario** de ejemplo en `index.astro:32`. No es una referencia real. |
| `/m/j480.jpeg`, `/m/j960.jpeg` | scan de refs rotas | Fixtures de `src/lib/images.test.ts:16-19`, no rutas de `public/`. |

---

## Siguiente paso

**Estado: esperando tu aprobación del reporte. No se ha borrado nada todavía.**

Cuando apruebes, ejecutar la Fase 4 en este orden:

- **Lote 1 — dependencia `marked`** (1 elemento)
  Quitar la línea de `web/package.json:17` → regenerar `pnpm-lock.yaml` con `pnpm --dir web install` (**no a mano**; ojo: pnpm no está instalado en esta máquina, hay que instalarlo o hacerlo donde sí esté) → `npm test` + `npm run build` → commit `chore(dead-code): elimina la dependencia marked de web`.
  *Riesgo:* `package-lock.json` también quedaría desincronizado; resolver antes el Lote 4 o aceptar la divergencia temporal.

- **Lote 2 — `const year` en `Footer.astro`** (1 elemento)
  Borrar `web/src/components/Footer.astro:2` (y el bloque `---` queda vacío, dejarlo así) → `npm test` + `npm run build` → commit `chore(dead-code): elimina const year sin usar en Footer`.

- **Lote 3 — `.DS_Store`** (6 elementos)
  `git rm --cached` de los 6 + borrado en disco. **Confirmar antes** si se añade `.DS_Store` a `.gitignore` en el mismo commit (es config, no borrado). → `npm run build` y verificar que `dist/` ya no los contiene → commit `chore(dead-code): elimina .DS_Store versionados del sitio publicado`.

- **Lote 4 — decisiones pendientes tuyas, no ejecutar sin respuesta:**
  1. ¿Borro las 7 imágenes huérfanas (~1.6 MB)? Son contenido, la curaduría es tuya.
  2. ¿Elimino `web/package-lock.json` para quedarnos solo con pnpm (lo que usa CI)?
  3. ¿Qué hago con `TuVertical.jpg` (referencia rota en `dango.astro:47`) — se sube el archivo o se quita el `<img>`? Es un arreglo funcional, fuera del alcance "solo eliminar".
  4. ¿`/proyectos/syle` (autodescrita "PAGINA DE PRUEBA") debe seguir publicada y enlazada desde el home?

- **Sesión futura — workspace Rust**
  Requiere instalar `rustup` + toolchain stable (+ `nasm`, que el README marca como prerequisito del encoder AVIF). Empezar por Fase 0/1 sobre los 6 crates. **Recordar forzar `--workspace` e incluir `admin/` (wasm32)**, que está fuera de `default-members` y cuya omisión produciría falsos positivos en `types/`.
