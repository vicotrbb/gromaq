use std::path::PathBuf;

#[cfg(test)]
mod tests;

pub(super) fn first_existing_font_path(candidates: Vec<PathBuf>) -> Option<PathBuf> {
    candidates.into_iter().find(|path| path.exists())
}

pub(super) fn default_monospace_font_candidate_paths() -> Vec<PathBuf> {
    let mut candidates = font_search_candidates(DEFAULT_PREFERRED_MONO_FONT_FILES);
    candidates.extend(DEFAULT_MONOSPACE_FONT_CANDIDATES.iter().map(PathBuf::from));
    candidates
}

pub(super) fn named_font_candidate_paths(font_family: &str) -> Option<Vec<PathBuf>> {
    let files = match normalized_font_family_name(font_family).as_str() {
        "meslolgsnf" | "meslolgsnerdfont" | "meslo" => MESLO_LGS_FONT_FILES,
        "jetbrainsmono" | "jetbrainsmononerdfont" => JETBRAINS_MONO_FONT_FILES,
        "cascadiamono" | "caskaydiacovenerdfont" => CASCADIA_MONO_FONT_FILES,
        "iosevkaterm" | "iosevka" => IOSEVKA_TERM_FONT_FILES,
        "geistmono" | "geistmononerdfont" => GEIST_MONO_FONT_FILES,
        "monaspaceneon" | "monaspace" => MONASPACE_NEON_FONT_FILES,
        "firacode" | "firacodenerdfont" => FIRA_CODE_FONT_FILES,
        "hack" | "hacknerdfont" => HACK_FONT_FILES,
        "sfmono" => SF_MONO_FONT_FILES,
        "menlo" => MENLO_FONT_FILES,
        _ => return None,
    };
    Some(font_search_candidates(files))
}

pub(super) fn fallback_font_paths() -> Vec<PathBuf> {
    DEFAULT_FALLBACK_FONT_CANDIDATES
        .iter()
        .map(PathBuf::from)
        .filter(|path| path.exists())
        .collect()
}

fn normalized_font_family_name(font_family: &str) -> String {
    font_family
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn font_search_candidates(file_names: &[&str]) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for root in font_search_roots() {
        for file_name in file_names {
            candidates.push(root.join(file_name));
        }
    }
    candidates
}

fn font_search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        roots.push(PathBuf::from(home).join("Library/Fonts"));
    }
    roots.extend([
        PathBuf::from("/Library/Fonts"),
        PathBuf::from("/System/Library/Fonts"),
        PathBuf::from("/opt/homebrew/share/fonts"),
        PathBuf::from("/usr/local/share/fonts"),
        PathBuf::from("/usr/share/fonts/truetype"),
        PathBuf::from("/usr/share/fonts/opentype"),
    ]);
    roots
}

const DEFAULT_PREFERRED_MONO_FONT_FILES: &[&str] = &[
    "JetBrainsMonoNerdFont-Regular.ttf",
    "JetBrainsMonoNLNerdFont-Regular.ttf",
    "JetBrainsMono-Regular.ttf",
    "MesloLGS NF Regular.ttf",
    "CaskaydiaCoveNerdFont-Regular.ttf",
    "CascadiaMono.ttf",
    "IosevkaTerm-Regular.ttf",
    "GeistMonoNerdFont-Regular.otf",
    "GeistMono-Regular.otf",
    "MonaspaceNeon-Regular.otf",
    "FiraCodeNerdFont-Regular.ttf",
    "FiraCode-Regular.ttf",
    "HackNerdFont-Regular.ttf",
    "Hack-Regular.ttf",
    "SFNSMono.ttf",
    "Menlo.ttc",
];

const DEFAULT_MONOSPACE_FONT_CANDIDATES: &[&str] = &[
    "/System/Library/Fonts/SFNSMono.ttf",
    "/System/Library/Fonts/Menlo.ttc",
    "/System/Library/Fonts/Supplemental/Courier New.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    "/usr/share/fonts/dejavu-sans-fonts/DejaVuSansMono.ttf",
    "/usr/share/fonts/truetype/liberation2/LiberationMono-Regular.ttf",
    "/usr/share/fonts/liberation/LiberationMono-Regular.ttf",
    "/usr/share/fonts/truetype/noto/NotoSansMono-Regular.ttf",
];

const DEFAULT_FALLBACK_FONT_CANDIDATES: &[&str] = &[
    "/System/Library/Fonts/Apple Symbols.ttf",
    "/usr/share/fonts/truetype/noto/NotoSansSymbols2-Regular.ttf",
    "/usr/share/fonts/truetype/noto/NotoSansSymbols-Regular.ttf",
    "/usr/share/fonts/noto/NotoSansSymbols2-Regular.ttf",
    "/usr/share/fonts/noto/NotoSansSymbols-Regular.ttf",
    "/System/Library/Fonts/Apple Color Emoji.ttc",
    "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf",
];

const JETBRAINS_MONO_FONT_FILES: &[&str] = &[
    "JetBrainsMonoNerdFont-Regular.ttf",
    "JetBrainsMonoNLNerdFont-Regular.ttf",
    "JetBrainsMono-Regular.ttf",
];

const MESLO_LGS_FONT_FILES: &[&str] = &["MesloLGS NF Regular.ttf"];

const CASCADIA_MONO_FONT_FILES: &[&str] = &[
    "CaskaydiaCoveNerdFont-Regular.ttf",
    "CascadiaMono.ttf",
    "CascadiaCode.ttf",
];

const IOSEVKA_TERM_FONT_FILES: &[&str] = &[
    "IosevkaTerm-Regular.ttf",
    "IosevkaTermNerdFont-Regular.ttf",
    "Iosevka-Regular.ttc",
];

const GEIST_MONO_FONT_FILES: &[&str] = &[
    "GeistMonoNerdFont-Regular.otf",
    "GeistMono-Regular.otf",
    "GeistMono-Regular.ttf",
];

const MONASPACE_NEON_FONT_FILES: &[&str] = &[
    "MonaspaceNeon-Regular.otf",
    "MonaspaceNeon-Regular.ttf",
    "MonaspaceNeonVarVF[wght,wdth,slnt].ttf",
];

const FIRA_CODE_FONT_FILES: &[&str] = &[
    "FiraCodeNerdFont-Regular.ttf",
    "FiraCode-Regular.ttf",
    "Fira Code Regular Nerd Font Complete.ttf",
];

const HACK_FONT_FILES: &[&str] = &[
    "HackNerdFont-Regular.ttf",
    "Hack-Regular.ttf",
    "Hack Regular Nerd Font Complete.ttf",
];

const SF_MONO_FONT_FILES: &[&str] = &["SFNSMono.ttf", "SFNSMonoItalic.ttf"];

const MENLO_FONT_FILES: &[&str] = &["Menlo.ttc"];
