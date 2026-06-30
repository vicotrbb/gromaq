# Gromaq GitHub Pages Landing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and verify a polished static GitHub Pages landing page for Gromaq `v0.2.1`, including a real native-app terminal recording.

**Architecture:** Add a deployable static site under `site/` with plain HTML, CSS, and small vanilla JavaScript for copy buttons. Keep release claims tied to README/release docs, add project-policy guards so the Pages surface stays deployable, and publish only the `site/` artifact through GitHub Pages.

**Tech Stack:** Static HTML/CSS/JS, Node for local link checks, GitHub Actions Pages deployment, existing Gromaq image assets, macOS `screencapture`/Swift/`ffmpeg` for real native-app recording.

---

## Reference Inputs

- Design spec: `documentation/superpowers/specs/2026-06-30-gromaq-github-pages-landing-design.md`
- Design context: `.impeccable.md`
- Visual reference: `https://guaracloud.github.io/e-navigator/`
- Existing assets:
  - `images/logos/logo-on-graphite.png`
  - `images/logos/logo-transparent.png`
  - `images/logos/logo-icon-512.png`
  - `images/screenshots/gromaq-welcome-preview.png`
  - `images/avatar/avatar-welcome.png`
- Existing proof helpers:
  - `scripts/capture-macos-window-proof.sh`
  - `scripts/prove-macos-live-input-manual.sh`
  - `scripts/prove-readme-welcome-preview.sh`

## File Structure

- Create: `site/index.html`
  - Semantic single-page landing content, SEO metadata, video/poster media, install commands, docs links.
- Create: `site/styles.css`
  - Warm off-white visual system, responsive layout, terminal media frame, command blocks, focus/reduced-motion states.
- Create: `site/scripts.js`
  - Copy buttons only. No framework, no analytics, no runtime dependency.
- Create: `site/check-links.mjs`
  - Local deploy-artifact link checker that rejects escaping `../` links and missing local assets.
- Create: `site/assets/*`
  - Deployable copies of required images and terminal recording.
- Create: `scripts/capture-pages-terminal-recording.sh`
  - macOS helper for real native-app recording frames/video, with clear permission guidance.
- Create: `.github/workflows/pages.yml`
  - GitHub Pages deployment from `site/` only.
- Create: `tests/project_policy/pages.rs`
  - Policy tests for required site files, deployment-safe links, media fallback, and Pages workflow.
- Modify: `tests/project_policy.rs`
  - Add the `pages` module.
- Modify: `tests/project_policy/native.rs`
  - Permit only the new static-site JavaScript files while preserving the native-app no-web-runtime rule.
- Modify: `tests/project_policy/ci.rs`
  - Add syntax checks for the Pages helper script and local link checker.
- Modify: `README.md`
  - Add the GitHub Pages URL after local verification.

## Task 1: Add Pages Policy Guards

**Files:**
- Create: `tests/project_policy/pages.rs`
- Modify: `tests/project_policy.rs`
- Modify: `tests/project_policy/native.rs`
- Modify: `tests/project_policy/ci.rs`

- [x] **Step 1: Add the project-policy module**

Add this line to `tests/project_policy.rs` after the `native` module:

```rust
#[path = "project_policy/pages.rs"]
mod pages;
```

- [x] **Step 2: Create failing Pages policy tests**

Create `tests/project_policy/pages.rs` with:

```rust
use std::{fs, path::Path};

use super::support::relative_path;

const REQUIRED_SITE_FILES: &[&str] = &[
    "site/index.html",
    "site/styles.css",
    "site/scripts.js",
    "site/check-links.mjs",
    "site/assets/logo-on-graphite.png",
    "site/assets/logo-transparent.png",
    "site/assets/logo-icon-512.png",
    "site/assets/gromaq-welcome-preview.png",
    "site/assets/gromaq-terminal-recording.webm",
    ".github/workflows/pages.yml",
    "scripts/capture-pages-terminal-recording.sh",
];

#[test]
fn pages_site_keeps_required_deployable_files() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for relative in REQUIRED_SITE_FILES {
        assert!(
            root.join(relative).is_file(),
            "{relative} must exist for the GitHub Pages launch site"
        );
    }
}

#[test]
fn pages_site_keeps_real_terminal_media_contract() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let html_path = root.join("site/index.html");
    let html = fs::read_to_string(&html_path).unwrap();

    for marker in [
        "<video",
        "assets/gromaq-terminal-recording.webm",
        "poster=\"assets/gromaq-welcome-preview.png\"",
        "Native Rust GPU terminal",
        "GROMAQ_INSTALL_METHOD=release GROMAQ_VERSION=v0.2.1",
        "public alpha/beta",
        "Developer ID notarization is not claimed",
    ] {
        assert!(
            html.contains(marker),
            "{} must include `{marker}` for the proof-first launch page",
            relative_path(root, &html_path)
        );
    }
}

#[test]
fn pages_site_links_stay_inside_deploy_artifact() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let html_path = root.join("site/index.html");
    let html = fs::read_to_string(&html_path).unwrap();

    assert!(
        !html.contains("../"),
        "{} must not link outside the deployed site artifact",
        relative_path(root, &html_path)
    );
    assert!(
        html.contains("https://github.com/vicotrbb/gromaq/releases/tag/v0.2.1"),
        "{} must link directly to the published v0.2.1 release",
        relative_path(root, &html_path)
    );
}

#[test]
fn pages_workflow_publishes_site_directory_only() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow_path = root.join(".github/workflows/pages.yml");
    let workflow = fs::read_to_string(&workflow_path).unwrap();

    for marker in [
        "actions/configure-pages@v5",
        "actions/upload-pages-artifact@v3",
        "actions/deploy-pages@v4",
        "path: site",
        "pages: write",
        "id-token: write",
        "node site/check-links.mjs",
    ] {
        assert!(
            workflow.contains(marker),
            "{} must include `{marker}` for Pages deployment",
            relative_path(root, &workflow_path)
        );
    }
}
```

- [x] **Step 3: Allow only the static-site JavaScript files**

In `tests/project_policy/native.rs`, rename `ALLOWED_IMAGE_TOOLING_FILES` to `ALLOWED_JAVASCRIPT_FILES` and include the site files:

```rust
const ALLOWED_JAVASCRIPT_FILES: &[&str] = &[
    "images/avatar/generate.mjs",
    "images/logos/generate.mjs",
    "images/tools/gromaq-image-assets.mjs",
    "site/scripts.js",
    "site/check-links.mjs",
];
```

Update `is_forbidden_frontend_file` to use the renamed allowlist:

```rust
if ALLOWED_JAVASCRIPT_FILES.contains(&relative.as_str()) {
    return false;
}
```

- [x] **Step 4: Add CI markers for Pages syntax checks**

In `tests/project_policy/ci.rs`, add these entries to `REQUIRED_CI_COMMANDS`:

```rust
"sh -n scripts/capture-pages-terminal-recording.sh",
"node --check site/scripts.js",
"node --check site/check-links.mjs",
"node site/check-links.mjs",
```

- [x] **Step 5: Run policy test and verify it fails for missing site files**

Run:

```bash
cargo test --test project_policy pages_site_keeps_required_deployable_files
```

Expected: FAIL because `site/index.html` and the Pages workflow do not exist yet.

- [ ] **Step 6: Keep policy guards uncommitted until the site exists**

Do not commit the policy guards while they are intentionally failing. Keep the
changes in the working tree and commit them with the first implementation slice
that makes the new Pages policy tests pass.

## Task 2: Add Static Site Assets And HTML

**Files:**
- Create: `site/index.html`
- Create: `site/assets/logo-on-graphite.png`
- Create: `site/assets/logo-transparent.png`
- Create: `site/assets/logo-icon-512.png`
- Create: `site/assets/gromaq-welcome-preview.png`

- [x] **Step 1: Create the deploy asset directory**

```bash
mkdir -p site/assets
cp images/logos/logo-on-graphite.png site/assets/logo-on-graphite.png
cp images/logos/logo-transparent.png site/assets/logo-transparent.png
cp images/logos/logo-icon-512.png site/assets/logo-icon-512.png
cp images/screenshots/gromaq-welcome-preview.png site/assets/gromaq-welcome-preview.png
```

- [x] **Step 2: Create `site/index.html`**

Use this complete structure, then refine copy only if it remains factually tied to README/release docs:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
    <meta name="description" content="Gromaq is a native Rust GPU-rendered terminal emulator built for speed, correctness, and real command work.">
    <meta property="og:title" content="Gromaq - Native Rust GPU terminal">
    <meta property="og:description" content="Install the public alpha/beta v0.2.1 release for macOS and Linux, with proof-backed release boundaries.">
    <meta property="og:image" content="./assets/gromaq-welcome-preview.png">
    <meta property="og:type" content="website">
    <meta name="twitter:card" content="summary_large_image">
    <title>Gromaq - Native Rust GPU terminal</title>
    <link rel="icon" type="image/png" href="./assets/logo-icon-512.png">
    <link rel="preload" as="image" href="./assets/gromaq-welcome-preview.png">
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link
      href="https://fonts.googleapis.com/css2?family=Afacad+Flux:wght@500;600;700&amp;family=Atkinson+Hyperlegible:ital,wght@0,400;0,700;1,400&amp;family=Recursive+Mono:wght@500;600&amp;display=swap"
      rel="stylesheet"
    >
    <link rel="stylesheet" href="./styles.css">
  </head>
  <body>
    <a class="skip-link" href="#main">Skip to content</a>
    <header class="site-header">
      <nav class="nav" aria-label="Primary navigation">
        <a class="brand" href="./index.html" aria-label="Gromaq home">
          <img src="./assets/logo-transparent.png" alt="" width="48" height="48">
          <span>Gromaq</span>
        </a>
        <div class="nav-links">
          <a href="#works">Works</a>
          <a href="#install">Install</a>
          <a href="#proof">Proof</a>
          <a href="https://github.com/vicotrbb/gromaq">GitHub</a>
        </div>
      </nav>
    </header>

    <main id="main">
      <section class="hero section" aria-labelledby="hero-title">
        <div class="hero-copy">
          <p class="eyebrow">Native Rust GPU terminal</p>
          <h1 id="hero-title">Built for speed, correctness, and real command work.</h1>
          <p class="hero-lede">
            Gromaq is a native terminal emulator built with Rust, winit, wgpu,
            real PTYs, and proof-backed release discipline. Version 0.2.1 is
            installable now as a public alpha/beta foundation release.
          </p>
          <div class="hero-actions" role="group" aria-label="Primary actions">
            <a class="button button-primary" href="#install">Install v0.2.1</a>
            <a class="button button-secondary" href="https://github.com/vicotrbb/gromaq">GitHub</a>
            <a class="button button-ghost" href="https://github.com/vicotrbb/gromaq/releases/tag/v0.2.1">Release notes</a>
          </div>
          <ul class="trust-row" aria-label="Project traits">
            <li>native Rust</li>
            <li>wgpu renderer</li>
            <li>real PTY</li>
            <li>alpha/beta 0.2.1</li>
          </ul>
        </div>

        <figure class="terminal-showcase" aria-labelledby="terminal-caption">
          <video
            controls
            muted
            playsinline
            preload="metadata"
            poster="assets/gromaq-welcome-preview.png"
          >
            <source src="assets/gromaq-terminal-recording.webm" type="video/webm">
          </video>
          <figcaption id="terminal-caption">
            Real native app capture: launch, welcome screen, shell input, and command output.
          </figcaption>
        </figure>
      </section>

      <section id="install" class="section install-section" aria-labelledby="install-title">
        <div class="section-heading">
          <p class="eyebrow">Install</p>
          <h2 id="install-title">Install the v0.2.1 release.</h2>
          <p>Use the release installer for macOS or Linux. It verifies published checksum manifests by default.</p>
        </div>
        <div class="command-grid">
          <article class="command-panel">
            <h3>macOS or Linux release</h3>
            <p>No Rust required on the macOS app path. Linux installs the release tarball.</p>
            <div class="command-code">
              <button class="copy-button" type="button" data-copy-target="release-install">Copy</button>
              <pre><code id="release-install">curl -fsSL https://raw.githubusercontent.com/vicotrbb/gromaq/main/scripts/install.sh | GROMAQ_INSTALL_METHOD=release GROMAQ_VERSION=v0.2.1 sh</code></pre>
            </div>
          </article>
          <article class="command-panel">
            <h3>Source install</h3>
            <p>Contributor path for macOS and Linux systems with Rust stable installed.</p>
            <div class="command-code">
              <button class="copy-button" type="button" data-copy-target="source-install">Copy</button>
              <pre><code id="source-install">curl -fsSL https://raw.githubusercontent.com/vicotrbb/gromaq/main/scripts/install.sh | sh</code></pre>
            </div>
          </article>
        </div>
      </section>

      <section id="works" class="section" aria-labelledby="works-title">
        <div class="section-heading">
          <p class="eyebrow">What works today</p>
          <h2 id="works-title">A real terminal foundation, not a web shell.</h2>
        </div>
        <div class="proof-list">
          <article><h3>Terminal core</h3><p>Grid/state, scrollback, resize reflow, alternate screen, selection, copy, and clipboard boundaries.</p></article>
          <article><h3>ANSI and input</h3><p>SGR colors, cursor movement, mouse reporting, bracketed paste, and Unicode wide/emoji cluster handling.</p></article>
          <article><h3>Native runtime</h3><p>PTY shell startup, input/output pump, resize propagation, large output, keyboard/mouse mapping, and live config reload.</p></article>
          <article><h3>Renderer and packaging</h3><p>Swash rasterization, glyph atlas/cache, wgpu bootstrap, Linux tarballs, Debian packages, Arch metadata, macOS app zips, and checksums.</p></article>
        </div>
      </section>

      <section class="section packages-section" aria-labelledby="packages-title">
        <div class="section-heading">
          <p class="eyebrow">Packages</p>
          <h2 id="packages-title">Published release assets.</h2>
        </div>
        <ul class="package-list">
          <li><strong>macOS:</strong> Gromaq-macos-app.zip</li>
          <li><strong>Linux:</strong> gromaq-0.2.1-linux-x86_64.tar.gz</li>
          <li><strong>Debian:</strong> gromaq_0.2.1_amd64.deb</li>
          <li><strong>Arch:</strong> PKGBUILD, default.SRCINFO, gromaq.install</li>
          <li><strong>Checksums:</strong> SHA256SUMS-linux-x86_64, SHA256SUMS-macos-app</li>
        </ul>
        <a class="text-link" href="https://github.com/vicotrbb/gromaq/releases/tag/v0.2.1">Open v0.2.1 release</a>
      </section>

      <section id="proof" class="section proof-section" aria-labelledby="proof-title">
        <div class="section-heading">
          <p class="eyebrow">Proof boundaries</p>
          <h2 id="proof-title">Public alpha/beta means honest boundaries.</h2>
          <p>Gromaq 0.2.1 is installable and usable by early adopters. It is not a v1.0 daily-driver stability claim.</p>
        </div>
        <div class="boundary-grid">
          <article><strong>No v1.0 claim</strong><span>Compatibility and daily-driver proof are still expanding.</span></article>
          <article><strong>No notarization claim</strong><span>Developer ID notarization is not claimed for this release.</span></article>
          <article><strong>No 144 Hz hardware claim yet</strong><span>Deterministic budgets exist; hardware-backed acceptance remains open.</span></article>
        </div>
      </section>

      <section class="section docs-section" aria-labelledby="docs-title">
        <div class="section-heading">
          <p class="eyebrow">Docs</p>
          <h2 id="docs-title">Inspect the proof trail.</h2>
        </div>
        <div class="doc-links">
          <a href="https://github.com/vicotrbb/gromaq#readme">README</a>
          <a href="https://github.com/vicotrbb/gromaq/blob/main/documentation/release.md">Release docs</a>
          <a href="https://github.com/vicotrbb/gromaq/blob/main/documentation/compatibility.md">Compatibility</a>
          <a href="https://github.com/vicotrbb/gromaq/blob/main/documentation/benchmarks.md">Benchmarks</a>
          <a href="https://github.com/vicotrbb/gromaq/blob/main/ROADMAP.md">Roadmap</a>
          <a href="https://github.com/vicotrbb/gromaq/blob/main/CONTRIBUTING.md">Contribute</a>
        </div>
      </section>
    </main>

    <footer class="site-footer">
      <span>Gromaq Terminal</span>
      <a href="https://github.com/vicotrbb/gromaq">Source on GitHub</a>
    </footer>
    <script src="./scripts.js" defer></script>
  </body>
</html>
```

- [x] **Step 3: Run the failing media-contract test**

Run:

```bash
cargo test --test project_policy pages_site_keeps_real_terminal_media_contract
```

Expected: FAIL because `site/assets/gromaq-terminal-recording.webm` is not captured yet.

- [ ] **Step 4: Keep HTML and image assets uncommitted until policy passes**

Do not commit yet because the recording, CSS, JavaScript, and Pages workflow
are still required for the new project-policy checks to pass.

## Task 3: Implement Impeccable Visual System

**Files:**
- Create: `site/styles.css`

- [x] **Step 1: Create `site/styles.css`**

Implement these required design constraints:

```css
:root {
  --font-display: "Afacad Flux", "Trebuchet MS", sans-serif;
  --font-body: "Atkinson Hyperlegible", "Aptos", sans-serif;
  --font-code: "Recursive Mono", "SFMono-Regular", ui-monospace, monospace;

  --paper: oklch(97% 0.014 95);
  --paper-raised: oklch(94% 0.018 95);
  --ink: oklch(17% 0.018 282);
  --ink-muted: oklch(41% 0.022 282);
  --line: oklch(82% 0.025 95);
  --violet: oklch(50% 0.22 300);
  --violet-soft: oklch(90% 0.055 300);
  --cyan: oklch(62% 0.115 220);
  --graphite: oklch(14% 0.018 282);
  --graphite-2: oklch(20% 0.025 282);

  --space-2xs: 4px;
  --space-xs: 8px;
  --space-sm: 12px;
  --space-md: 16px;
  --space-lg: 24px;
  --space-xl: 32px;
  --space-2xl: 48px;
  --space-3xl: 64px;
  --space-4xl: 96px;
  --radius-sm: 6px;
  --radius-md: 8px;
  --ease-out-quart: cubic-bezier(0.25, 1, 0.5, 1);
}
```

Keep these rules in the final CSS:

```css
body {
  margin: 0;
  min-width: 320px;
  color: var(--ink);
  background: var(--paper);
  font-family: var(--font-body);
  font-size: 1rem;
  line-height: 1.6;
}

h1,
h2,
h3 {
  margin: 0;
  font-family: var(--font-display);
  letter-spacing: 0;
}

h1 {
  max-width: 880px;
  font-size: clamp(3.25rem, 8vw, 6.8rem);
  line-height: 0.96;
}

.hero {
  display: grid;
  grid-template-columns: minmax(0, 0.92fr) minmax(360px, 1.08fr);
  align-items: center;
  gap: clamp(var(--space-xl), 6vw, var(--space-4xl));
  min-height: calc(100svh - 82px);
}

.terminal-showcase video {
  display: block;
  width: 100%;
  aspect-ratio: 16 / 9;
  border: 1px solid color-mix(in oklch, var(--violet), black 35%);
  border-radius: var(--radius-md);
  background: var(--graphite);
}

@media (max-width: 900px) {
  .hero {
    grid-template-columns: 1fr;
    min-height: auto;
  }
}

@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
    scroll-behavior: auto !important;
  }
}
```

Reject during review:

```bash
rg -n "background-clip:\\s*text|border-(left|right):\\s*[2-9]px|#000|#fff|linear-gradient\\([^)]*text" site/styles.css
```

Expected: no matches.

- [x] **Step 2: Run CSS anti-pattern scan**

Run:

```bash
rg -n "background-clip:\\s*text|border-(left|right):\\s*[2-9]px|#000|#fff|linear-gradient\\([^)]*text" site/styles.css
```

Expected: exit 1 with no matches.

- [x] **Step 3: Keep CSS in the working tree**

Do not commit yet. The Pages implementation should be committed once the local
site checks and project-policy suite pass.

## Task 4: Add Copy Buttons And Link Checker

**Files:**
- Create: `site/scripts.js`
- Create: `site/check-links.mjs`

- [x] **Step 1: Add copy-button behavior**

Create `site/scripts.js`:

```js
const copyButtons = document.querySelectorAll("[data-copy-target]");

copyButtons.forEach((button) => {
  const originalText = button.textContent;
  button.addEventListener("click", async () => {
    const targetId = button.getAttribute("data-copy-target");
    const target = targetId ? document.getElementById(targetId) : null;
    if (!target) return;

    try {
      await navigator.clipboard.writeText(target.textContent.trim());
      button.textContent = "Copied";
      window.setTimeout(() => {
        button.textContent = originalText;
      }, 1600);
    } catch {
      button.textContent = "Select command";
      window.setTimeout(() => {
        button.textContent = originalText;
      }, 2200);
    }
  });
});
```

- [x] **Step 2: Add deploy-artifact link checker**

Create `site/check-links.mjs`:

```js
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, normalize } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(fileURLToPath(import.meta.url));
const htmlPath = join(root, "index.html");
const html = readFileSync(htmlPath, "utf8");
const attributePattern = /\b(?:href|src|poster)=["']([^"']+)["']/g;
const failures = [];

for (const match of html.matchAll(attributePattern)) {
  const value = match[1];
  if (
    value.startsWith("#") ||
    value.startsWith("http://") ||
    value.startsWith("https://") ||
    value.startsWith("mailto:")
  ) {
    continue;
  }
  if (value.startsWith("../")) {
    failures.push(`Escaping deploy artifact: ${value}`);
    continue;
  }
  const withoutAnchor = value.split("#")[0];
  if (!withoutAnchor) continue;
  const localPath = normalize(join(root, withoutAnchor));
  if (!localPath.startsWith(root)) {
    failures.push(`Escaping deploy artifact: ${value}`);
    continue;
  }
  if (!existsSync(localPath)) {
    failures.push(`Missing local asset: ${value}`);
  }
}

if (failures.length > 0) {
  console.error(failures.join("\n"));
  process.exit(1);
}

console.log("Pages link check: ok");
```

- [x] **Step 3: Run JavaScript syntax and link checks**

Run:

```bash
node --check site/scripts.js
node --check site/check-links.mjs
node site/check-links.mjs
```

Expected: syntax checks pass; link check fails only until the terminal recording file exists.

Current result: syntax checks pass and the link checker passes with the
explicit blocker fallback because no fake recording asset is linked.

- [x] **Step 4: Keep JS and checker in the working tree**

Do not commit yet. The checker is expected to pass after the recording asset is
captured in Task 5.

## Task 5: Capture Real Native Terminal Recording

**Files:**
- Create: `scripts/capture-pages-terminal-recording.sh`
- Create: `site/assets/gromaq-terminal-recording.webm`

- [x] **Step 1: Add the recording helper**

Create `scripts/capture-pages-terminal-recording.sh`. It should:

- Require Darwin.
- Require `swift`, `screencapture`, `ffmpeg`, and `osascript`.
- Build `target/debug/gromaq`.
- Launch `target/debug/gromaq` with a controlled config in `target/pages-recording/gromaq-pages.toml`.
- Use `osascript` to focus Gromaq and type:
  - `gromaq --version`
  - `pwd`
  - `printf 'frame proof line %03d\n' 1 2 3 4 5`
- Capture the Gromaq window by id into numbered PNG frames.
- Encode `site/assets/gromaq-terminal-recording.webm` with `ffmpeg`.
- Write `target/pages-recording/summary.txt`.

Use the existing Swift window lookup approach from `scripts/capture-macos-window-proof.sh`; do not invent a separate window matching policy.

- [x] **Step 2: Run shell syntax check**

Run:

```bash
sh -n scripts/capture-pages-terminal-recording.sh
```

Expected: PASS.

- [x] **Step 3: Capture the real recording**

Run:

```bash
scripts/capture-pages-terminal-recording.sh
```

Expected:

- `site/assets/gromaq-terminal-recording.webm` exists.
- `target/pages-recording/summary.txt` says `Gromaq Pages terminal recording: ok`.
- The recording shows the current native app window, welcome screen, shell input, and command output.

If macOS Screen Recording or Accessibility permission blocks capture or keystrokes, grant permission to the terminal application and rerun the same command. If permission remains blocked, keep the poster fallback intact and report the exact blocker.

Current result: blocked by macOS Screen Recording permission preflight returning
`false`; poster fallback and explicit blocker copy are active.

- [x] **Step 4: Verify media file metadata**

Run:

```bash
ffprobe -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 site/assets/gromaq-terminal-recording.webm
```

Expected: duration is between `10.0` and `16.0` seconds.

Current result: not applicable because macOS Screen Recording permission blocked
capture before a media file could be produced.

- [x] **Step 5: Run site link check**

Run:

```bash
node site/check-links.mjs
```

Expected: `Pages link check: ok`.

- [x] **Step 6: Keep recording helper and media in the working tree**

Do not commit yet. Commit the full site after the Pages workflow and policy
checks pass.

## Task 6: Add GitHub Pages Workflow

**Files:**
- Create: `.github/workflows/pages.yml`

- [x] **Step 1: Create Pages workflow**

Create `.github/workflows/pages.yml`:

```yaml
name: GitHub Pages

on:
  push:
    branches:
      - main
  workflow_dispatch:

permissions:
  contents: read
  pages: write
  id-token: write

concurrency:
  group: pages
  cancel-in-progress: false

jobs:
  deploy:
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - uses: actions/setup-node@v5
        with:
          node-version: "22"
      - run: node --check site/scripts.js
      - run: node --check site/check-links.mjs
      - run: node site/check-links.mjs
      - uses: actions/configure-pages@v5
        with:
          enablement: true
      - uses: actions/upload-pages-artifact@v3
        with:
          path: site
      - id: deployment
        uses: actions/deploy-pages@v4
```

- [x] **Step 2: Run workflow and policy checks**

Run:

```bash
node --check site/scripts.js
node --check site/check-links.mjs
node site/check-links.mjs
cargo test --test project_policy pages_workflow_publishes_site_directory_only
```

Expected: all pass.

- [ ] **Step 3: Commit the complete verified Pages slice**

```bash
git add tests/project_policy.rs tests/project_policy/pages.rs tests/project_policy/native.rs tests/project_policy/ci.rs
git add site scripts/capture-pages-terminal-recording.sh .github/workflows/pages.yml
git commit -m "feat: add gromaq pages launch site"
```

## Task 7: Update README And Final Validation

**Files:**
- Modify: `README.md`

- [x] **Step 1: Add launch-site link to README**

Add this line near the top after the project description:

```markdown
Launch site: `https://vicotrbb.github.io/gromaq/`
```

- [x] **Step 2: Run local static server**

Run:

```bash
python3 -m http.server 4173 --directory site
```

Expected: server starts at `http://localhost:4173/`.

- [x] **Step 3: Visually inspect responsive states with browser automation**

Open `http://localhost:4173/` and verify:

- Desktop hero fits without text/media overlap.
- Mobile stacks hero copy before terminal media.
- The warm off-white background matches the approved reference direction.
- The video poster is visible before playback.
- Copy buttons update to `Copied`.
- Focus rings are visible from keyboard navigation.

- [ ] **Step 4: Run final verification**

Run:

```bash
sh -n scripts/capture-pages-terminal-recording.sh
node --check site/scripts.js
node --check site/check-links.mjs
node site/check-links.mjs
cargo test --test project_policy
git diff --check
git diff --cached --check
```

Expected: all pass.

- [ ] **Step 5: Commit README update and final fixes**

```bash
git add README.md
git commit -m "docs: link gromaq launch site"
```

## Completion Criteria

- `site/` contains a complete deployable landing page.
- The hero uses warm off-white page styling and real native terminal media.
- The terminal recording is captured from the current native app.
- `node site/check-links.mjs` passes.
- `cargo test --test project_policy` passes.
- The GitHub Pages workflow publishes only `site/`.
- No forbidden `docs/` tree exists.
- No fake terminal animation is used.
