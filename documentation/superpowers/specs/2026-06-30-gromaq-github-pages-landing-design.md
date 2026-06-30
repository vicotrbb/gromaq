# Gromaq GitHub Pages Landing Page Design

## Status

Approved direction: proof-first product launch page inspired by the
E-Navigator GitHub Pages site, adapted for Gromaq's terminal product identity.

This is a design spec only. Implementation should follow after explicit user
review.

## Objective

Create a simple but polished GitHub Pages landing page for Gromaq that is
shareable with early adopters, open-source contributors, and terminal-heavy
developers. The page should advertise the `v0.2.1` public alpha/beta release
without overstating maturity.

The site should make three points immediately:

- Gromaq is a native Rust GPU-rendered terminal emulator.
- The current release is installable now on supported macOS and Linux paths.
- The project is proof-driven, with clear boundaries around what is and is not
  proven yet.

## Audience

Primary users are developers, terminal-heavy operators, and open-source
contributors who care about native performance, PTY correctness, terminal
compatibility, packaging quality, and honest release posture.

The first visit context is likely a link from GitHub, social sharing, README,
or release notes. The page must be understandable in under 30 seconds and
credible enough to send visitors to install, inspect the release, or contribute.

## Visual Direction

Use a warm off-white page shell inspired by the E-Navigator site rather than a
black terminal page. Gromaq should still feel terminal-native through media,
contrast, command blocks, and restrained violet accents.

Core palette direction:

- Background: warm ivory/off-white, tinted rather than pure white.
- Primary text: near-graphite, not pure black.
- Accent: Gromaq violet, used sparingly for brand emphasis and focus states.
- Secondary accent: muted terminal cyan, reserved for terminal/proof details.
- Media surfaces: dark graphite terminal frames containing the real app
  recording.
- Panels: subtle warm raised surfaces for commands and proof summaries.

Tone: precise, calm, capable, and public-release ready. Avoid generic dark
developer-site tropes, neon-on-black defaults, gradient text, glassmorphism, and
card-heavy repetition.

## Typography

Use a distinctive display face for large headings, a highly readable body face,
and a monospace face only where the content is genuinely code, command, or
terminal output.

Brand voice words for font selection:

- precise
- serious
- sharpened

Reject common default choices from the design skill guidance, including Inter,
IBM Plex, Space Grotesk, Space Mono, DM Sans, Outfit, and similar overused
developer-site fonts. The heading should feel engineered and memorable without
turning the page into a novelty poster.

## Content Architecture

### Header

Sticky or top-positioned navigation with:

- Gromaq logo and name.
- Links: Works, Install, Proof, GitHub.

The nav should mirror the clarity of E-Navigator while using Gromaq assets.

### Hero

The hero should combine direct launch copy with the real product signal.

Recommended layout:

```text
[Gromaq logo]                         Works  Install  Proof  GitHub

Native Rust GPU terminal.
Built for speed, correctness,
and real daily command work.

[Install v0.2.1] [GitHub] [Release notes]

[native rust] [wgpu] [real PTY] [alpha/beta 0.2.1]

                  +--------------------------------------+
                  | real Gromaq app recording            |
                  | launch -> welcome -> shell -> output |
                  +--------------------------------------+
```

The H1 should name the category and product value plainly. Supporting copy
should state that Gromaq is native Rust, GPU-rendered, and currently a public
alpha/beta foundation release.

Primary action: install `v0.2.1`.

Secondary actions: GitHub and release notes.

### Real Terminal Recording

The hero media must be a real recording of the current native app, not a mockup
or scripted web animation.

Recording target:

- 10 to 15 seconds.
- Launch Gromaq.
- Show the welcome screen.
- Type real shell input.
- Show command output.
- Include scroll or selection if it is visually clean and does not distract.

The page should prefer a compressed video format such as `.webm` or `.mp4`,
with a static poster fallback using `images/screenshots/gromaq-welcome-preview.png`.
The media must not block the page from being useful if autoplay is disabled.

### Install Section

Show macOS and Linux release install commands prominently, with copy buttons.

Use the release installer commands from the README:

```bash
curl -fsSL https://raw.githubusercontent.com/vicotrbb/gromaq/main/scripts/install.sh | GROMAQ_INSTALL_METHOD=release GROMAQ_VERSION=v0.2.1 sh
```

Also include source install as a contributor path, but do not make it the first
choice.

### What Works Today

Summarize proven current functionality from the README in tightly edited
language:

- Terminal grid/state, scrollback, resize reflow, alternate screen, selection,
  copy, and clipboard boundaries.
- ANSI/VT parsing for colors, attributes, cursor movement, mouse reporting,
  bracketed paste, and Unicode wide/emoji cluster handling.
- Native PTY runtime with shell startup, input/output pump, resize propagation,
  large output, and scripted workflow proof.
- Native `winit` app lifecycle, keyboard/mouse mapping, live config reload,
  zoom, frame scheduling, FPS overlay, startup welcome screen, and generated
  logo icon.
- Swash-backed font rasterization, glyph atlas/cache, `wgpu` bootstrap, GPU
  smoke tests, and presentable window-surface glyph-frame path.
- Release automation for Linux tarballs, Debian packages, Arch metadata, macOS
  app zips, and checksum manifests.

This section should read as proof-backed capability, not as a feature grid full
of vague marketing claims.

### Packages

Show current release assets:

- macOS universal app bundle: `Gromaq-macos-app.zip`.
- Linux tarball: `gromaq-0.2.1-linux-x86_64.tar.gz`.
- Debian package: `gromaq_0.2.1_amd64.deb`.
- Arch metadata: `PKGBUILD`, `default.SRCINFO`, `gromaq.install`.
- Checksum manifests: `SHA256SUMS-linux-x86_64`,
  `SHA256SUMS-macos-app`.

Link the section to the GitHub `v0.2.1` release.

### Proof Boundaries

Keep the page honest about release posture:

- Public alpha/beta foundation release, not v1.0.
- Not a daily-driver stability claim.
- macOS app is ad-hoc signed unless future release proof says otherwise.
- Developer ID notarization is not claimed.
- Hardware-backed 144 Hz acceptance is not yet proven.
- Compatibility matrix is still expanding.
- Accepted live desktop screenshot proof remains a tracked gap until closed.

These should be framed as disciplined project boundaries, not apologies.

### Documentation And Contribution

Provide direct links to:

- GitHub repository.
- `v0.2.1` release.
- README.
- `documentation/release.md`.
- `documentation/compatibility.md`.
- `documentation/benchmarks.md`.
- `ROADMAP.md`.
- `CONTRIBUTING.md`.

## Interaction Requirements

- Copy buttons for install commands.
- Keyboard-visible focus states.
- Navigation anchors for major sections.
- Media controls or accessible fallback for the terminal recording.
- Reduced-motion handling for page load animations and autoplay behavior.
- No critical content hidden behind hover-only interactions.

## Technical Plan

Use a small static site under `site/`, suitable for GitHub Pages deployment.
Prefer plain HTML, CSS, and small vanilla JavaScript for copy buttons. Avoid
frameworks and build tooling unless implementation discovers a hard need.

Expected structure:

```text
site/
  index.html
  styles.css
  scripts.js
  assets/
    logo-on-graphite.png
    logo-transparent.png
    gromaq-welcome-preview.png
    gromaq-terminal-recording.webm
    gromaq-terminal-recording.mp4
```

Add a GitHub Pages workflow that publishes only the deployable `site/` artifact.
Use deployment-safe links; avoid repo-relative `../` links that work locally but
break on Pages.

## Validation

Implementation should verify:

- Static link check for local files and external URLs where practical.
- No deploy artifact links escape `site/`.
- HTML and CSS parse cleanly.
- Copy buttons work without console errors.
- Page renders correctly at desktop, tablet, and mobile widths.
- Terminal recording loads, has a poster fallback, and remains optional if
  autoplay is blocked.
- GitHub Pages workflow is syntactically valid.
- Existing Rust release proof and project policy tests are not disturbed by the
  site addition.

## Open Implementation Notes

- The real terminal recording must be captured from the current native app
  before finalizing the hero media.
- If recording capture is blocked, implementation should use the existing
  welcome preview as a temporary poster and report the recording blocker rather
  than replacing it with a fake animation.
- Repository homepage metadata can be updated to the eventual GitHub Pages URL
  after the page is deployed and verified.
