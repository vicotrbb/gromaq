const SECTION_RULE_WIDTH: usize = 36;
const WELCOME_GAP_WIDTH: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct WelcomeStyle {
    pub(super) title: [u8; 3],
    pub(super) value: [u8; 3],
    pub(super) section: [u8; 3],
}

pub(super) enum WelcomeEntry<'a> {
    Metric { label: &'static str, value: &'a str },
    Section(&'static str),
}

pub(super) fn push_welcome_row(
    output: &mut String,
    cols: u16,
    avatar: &str,
    entry: WelcomeEntry<'_>,
    style: WelcomeStyle,
) {
    output.push_str(avatar);
    let stat_budget = stat_budget(cols, avatar);
    if stat_budget == 0 {
        output.push_str("\r\n");
        return;
    }
    output.push_str(&" ".repeat(WELCOME_GAP_WIDTH.min(stat_budget)));
    let stat_budget = stat_budget.saturating_sub(WELCOME_GAP_WIDTH);
    let stat = match entry {
        WelcomeEntry::Metric { label, value } => metric_line(style, label, value, stat_budget),
        WelcomeEntry::Section(label) => section_line(style, label, stat_budget),
    };
    push_ansi_clipped(output, &stat, stat_budget);
    output.push_str("\r\n");
}

pub(super) fn ansi_visible_width(value: &str) -> usize {
    let mut width = 0;
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            width += 1;
        }
    }
    width
}

fn stat_budget(cols: u16, avatar: &str) -> usize {
    usize::from(cols).saturating_sub(ansi_visible_width(avatar))
}

fn metric_line(style: WelcomeStyle, label: &str, value: &str, max_width: usize) -> String {
    let prefix_width = 4 + label.len() + 2;
    let value = clipped_plain(value, max_width.saturating_sub(prefix_width));
    format!(
        "    {}{label}\x1b[0m  {}{value}\x1b[0m",
        bold_foreground(style.title),
        foreground(style.value)
    )
}

fn section_line(style: WelcomeStyle, label: &str, max_width: usize) -> String {
    format!(
        "{}{}\x1b[0m",
        foreground(style.section),
        section_header(label, max_width)
    )
}

fn section_header(label: &str, max_width: usize) -> String {
    let title = format!("  [ {label} ] ");
    if max_width <= title.len() {
        return clipped_plain(&title, max_width);
    }
    let rule_width = SECTION_RULE_WIDTH
        .min(max_width)
        .saturating_sub(title.len())
        .max(4);
    format!("{title}{}", "-".repeat(rule_width))
}

fn push_ansi_clipped(output: &mut String, value: &str, max_width: usize) {
    let mut visible = 0;
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            output.push(ch);
            for next in chars.by_ref() {
                output.push(next);
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
            continue;
        }
        if visible >= max_width {
            break;
        }
        output.push(ch);
        visible += 1;
    }
    output.push_str("\x1b[0m");
}

fn clipped_plain(value: &str, max_width: usize) -> String {
    value.chars().take(max_width).collect()
}

fn foreground([red, green, blue]: [u8; 3]) -> String {
    format!("\x1b[38;2;{red};{green};{blue}m")
}

fn bold_foreground([red, green, blue]: [u8; 3]) -> String {
    format!("\x1b[1;38;2;{red};{green};{blue}m")
}
