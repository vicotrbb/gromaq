pub(super) const REQUIRED_README_LAUNCH_MARKERS: &[&str] = &[
    "public alpha/beta terminal foundation release",
    "GROMAQ_INSTALL_METHOD=release GROMAQ_VERSION=v0.2.1",
    "Gromaq-macos-app.zip",
    "gromaq_0.2.1_amd64.deb",
    "`PKGBUILD`, `default.SRCINFO`, and",
    "Accepted live desktop screenshot proof",
    "Hardware-backed 144 Hz frame pacing proof",
    "Developer ID signed and notarized macOS distribution",
    "scripts/prove-local-ci-parity.sh",
    "documentation/release.md",
    "documentation/compatibility.md",
    "documentation/benchmarks.md",
    "SECURITY.md",
];

pub(super) const REQUIRED_RELEASE_DOC_MARKERS: &[(&str, &str)] = &[
    ("documentation/release.md", "GROMAQ_VERSION=v0.2.1"),
    ("documentation/release.md", "git tag -a v0.2.1"),
    (
        "documentation/release.md",
        "scripts/prove-github-release-publication.sh",
    ),
    (
        "documentation/release.md",
        "scripts/prove-github-release-install.sh",
    ),
    (
        "documentation/release.md",
        "scripts/prove-github-release-macos-install.sh",
    ),
    (
        "documentation/release.md",
        "gromaq-0.2.1-linux-x86_64.tar.gz",
    ),
    ("documentation/release.md", "gromaq_0.2.1_amd64.deb"),
    ("documentation/release.md", "Gromaq-macos-app.zip"),
    ("documentation/release.md", "default.SRCINFO"),
    ("documentation/release.md", "Historical v0.1.0 Proof"),
    ("documentation/compatibility.md", "public alpha/beta"),
    (
        "documentation/compatibility.md",
        "scripts/prove-current-host-compatibility.sh",
    ),
    (
        "documentation/compatibility.md",
        "accepted live desktop visual inspection/screenshot",
    ),
    (
        "documentation/compatibility.md",
        "default-startup tmux UI proof host",
    ),
    (
        "documentation/compatibility.md",
        "Developer ID signed/notarized macOS app distribution",
    ),
    (
        "documentation/benchmarks.md",
        "scripts/prove-local-ci-parity.sh",
    ),
    (
        "documentation/benchmarks.md",
        "cargo bench --bench parser_throughput -- --list",
    ),
];

pub(super) const REQUIRED_VISUAL_CONTRACT_DOC_MARKERS: &[(&str, &str)] = &[
    ("documentation/theme.md", "32 px font size"),
    ("documentation/theme.md", "44 px line height"),
    ("documentation/theme.md", "18 px automatic cell width"),
    ("documentation/theme.md", "gromaq-ghostty"),
    ("documentation/theme.md", "scripts/prove-welcome-preview.sh"),
    ("documentation/theme.md", "gromaq --theme-preview-config"),
    ("documentation/compatibility.md", "gromaq-ghostty"),
    ("documentation/compatibility.md", "avatar color pixels"),
    ("documentation/release.md", "36x17-sampled Braille avatar"),
];

pub(super) const REQUIRED_README_COMPLETION_GAP_MARKERS: &[&str] = &[
    "Accepted live desktop screenshot proof",
    "Hardware-backed 144 Hz frame pacing proof",
    "Wider compatibility matrix coverage",
    "Developer ID signed and notarized macOS distribution",
];
