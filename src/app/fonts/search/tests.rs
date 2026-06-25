use super::*;

#[test]
fn default_font_stack_prefers_polished_user_fonts_before_system_fallbacks() {
    let candidates = default_monospace_font_candidate_paths();
    let names = candidate_file_names(&candidates);

    let sf_mono_index = file_name_index(&names, "SFNSMono.ttf");
    assert_eq!(
        file_name_index(&names, "JetBrainsMonoNerdFont-Regular.ttf"),
        0
    );
    assert!(file_name_index(&names, "MesloLGS NF Regular.ttf") < sf_mono_index);
    assert!(file_name_index(&names, "GeistMonoNerdFont-Regular.otf") < sf_mono_index);
    assert!(file_name_index(&names, "MonaspaceNeon-Regular.otf") < sf_mono_index);
    assert!(file_name_index(&names, "FiraCodeNerdFont-Regular.ttf") < sf_mono_index);
    assert!(file_name_index(&names, "HackNerdFont-Regular.ttf") < sf_mono_index);
}

#[test]
fn named_font_resolution_normalizes_common_family_names() {
    assert_candidate_contains(
        "JetBrains Mono Nerd Font",
        "JetBrainsMonoNerdFont-Regular.ttf",
    );
    assert_candidate_contains("MesloLGS NF", "MesloLGS NF Regular.ttf");
    assert_candidate_contains("Geist Mono", "GeistMonoNerdFont-Regular.otf");
    assert_candidate_contains("Monaspace Neon", "MonaspaceNeon-Regular.otf");
    assert_candidate_contains("Fira Code Nerd Font", "FiraCodeNerdFont-Regular.ttf");
    assert_candidate_contains("Hack Nerd Font", "HackNerdFont-Regular.ttf");
    assert!(named_font_candidate_paths("Unmapped Mono").is_none());
}

fn assert_candidate_contains(family: &str, file_name: &str) {
    let candidates = named_font_candidate_paths(family).unwrap();
    let names = candidate_file_names(&candidates);

    assert!(names.iter().any(|name| name == file_name));
}

fn candidate_file_names(candidates: &[PathBuf]) -> Vec<String> {
    candidates
        .iter()
        .filter_map(|path| path.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .collect()
}

fn file_name_index(names: &[String], file_name: &str) -> usize {
    names.iter().position(|name| name == file_name).unwrap()
}
