/// Klyde Reader — a terminal book reader for Klyde.md
///
/// Controls:
///   ↓ / j / Space / Enter   next page
///   ↑ / k / Backspace        previous page
///   g                        go to first page
///   G                        go to last page
///   q / Esc                  quit

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{self, ClearType},
};
use std::{
    fs,
    io::{self, Write},
};
use textwrap::wrap;

const FILE: &str = "Klyde.md";
// Columns reserved for the left/right border padding
const H_PADDING: usize = 4; // 2 spaces each side inside the border
// Rows reserved for: top border + title bar + divider + bottom divider + status + bottom border
const V_OVERHEAD: usize = 6;

fn main() -> io::Result<()> {
    // ── Load file ──────────────────────────────────────────────────────────
    let raw = fs::read_to_string(FILE).unwrap_or_else(|_| {
        eprintln!("Cannot open '{}'. Place Klyde.md in the current directory.", FILE);
        std::process::exit(1);
    });

    // ── Render markdown lines into display lines ───────────────────────────
    let (cols, rows) = terminal::size()?;
    let inner_width = cols as usize - H_PADDING - 2; // 2 for the side borders
    let page_height = rows as usize - V_OVERHEAD;

    let display_lines = render_md(&raw, inner_width);
    let pages = paginate(&display_lines, page_height);
    let total_pages = pages.len().max(1);

    // ── Enter raw/alternate-screen mode ───────────────────────────────────
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;

    let mut page: usize = 0;
    draw_page(&mut stdout, &pages, page, total_pages, cols, rows)?;

    loop {
        if let Event::Key(KeyEvent { code, modifiers, .. }) = event::read()? {
            // Ctrl-C always quits
            if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
                break;
            }
            match code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Down
                | KeyCode::Char('j')
                | KeyCode::Char(' ')
                | KeyCode::Enter
                | KeyCode::PageDown => {
                    if page + 1 < total_pages {
                        page += 1;
                        draw_page(&mut stdout, &pages, page, total_pages, cols, rows)?;
                    }
                }
                KeyCode::Up
                | KeyCode::Char('k')
                | KeyCode::Backspace
                | KeyCode::PageUp => {
                    if page > 0 {
                        page -= 1;
                        draw_page(&mut stdout, &pages, page, total_pages, cols, rows)?;
                    }
                }
                KeyCode::Char('g') | KeyCode::Home => {
                    page = 0;
                    draw_page(&mut stdout, &pages, page, total_pages, cols, rows)?;
                }
                KeyCode::Char('G') | KeyCode::End => {
                    page = total_pages - 1;
                    draw_page(&mut stdout, &pages, page, total_pages, cols, rows)?;
                }
                _ => {}
            }
        }
    }

    // ── Restore terminal ───────────────────────────────────────────────────
    execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show)?;
    terminal::disable_raw_mode()?;
    Ok(())
}

// ── Rendering ──────────────────────────────────────────────────────────────

/// A line ready to print, with optional style hints.
#[derive(Clone)]
enum Line {
    Blank,
    H1(String),
    H2(String),
    H3(String),
    Rule,
    Body(String),
    Bold(String),  // lines that are entirely bold (e.g. **…** standalone)
    Code(String),  // inline code / code block lines
}

/// Turn raw markdown text into styled display lines, word-wrapped to `width`.
fn render_md(src: &str, width: usize) -> Vec<Line> {
    let mut out: Vec<Line> = Vec::new();
    let mut in_code_block = false;

    for raw_line in src.lines() {
        // Code fences
        if raw_line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            out.push(Line::Code(raw_line.to_string()));
            continue;
        }

        // Blank line
        if raw_line.trim().is_empty() {
            out.push(Line::Blank);
            continue;
        }

        // Horizontal rule
        if raw_line.trim() == "---" || raw_line.trim() == "***" || raw_line.trim() == "___" {
            out.push(Line::Rule);
            continue;
        }

        // Headings
        if let Some(h) = raw_line.strip_prefix("### ") {
            out.push(Line::H3(h.trim().to_string()));
            continue;
        }
        if let Some(h) = raw_line.strip_prefix("## ") {
            out.push(Line::Blank);
            out.push(Line::H2(h.trim().to_string()));
            out.push(Line::Blank);
            continue;
        }
        if let Some(h) = raw_line.strip_prefix("# ") {
            out.push(Line::Blank);
            out.push(Line::H1(h.trim().to_string()));
            out.push(Line::Blank);
            continue;
        }

        // Detect if the whole line is **bold** or __bold__
        let trimmed = raw_line.trim();
        if (trimmed.starts_with("**") && trimmed.ends_with("**") && trimmed.len() > 4)
            || (trimmed.starts_with("__") && trimmed.ends_with("__") && trimmed.len() > 4)
        {
            let inner = trimmed[2..trimmed.len() - 2].to_string();
            for wrapped in wrap(&inner, width) {
                out.push(Line::Bold(wrapped.to_string()));
            }
            continue;
        }

        // Regular body text — strip inline markdown markers for clean display
        let clean = strip_inline_md(raw_line.trim());
        for wrapped in wrap(&clean, width) {
            out.push(Line::Body(wrapped.to_string()));
        }
    }

    out
}

/// Strip inline markdown markers (bold, italic, inline code, links).
fn strip_inline_md(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        // Bold/italic: ** or __
        if i + 1 < chars.len()
            && ((chars[i] == '*' && chars[i + 1] == '*')
                || (chars[i] == '_' && chars[i + 1] == '_'))
        {
            i += 2;
            continue;
        }
        // Single * or _
        if chars[i] == '*' || chars[i] == '_' {
            i += 1;
            continue;
        }
        // Inline code `…`
        if chars[i] == '`' {
            i += 1;
            continue;
        }
        // Markdown link [text](url) → keep text
        if chars[i] == '[' {
            i += 1;
            let mut text = String::new();
            while i < chars.len() && chars[i] != ']' {
                text.push(chars[i]);
                i += 1;
            }
            // skip ](url)
            if i < chars.len() && chars[i] == ']' {
                i += 1;
                if i < chars.len() && chars[i] == '(' {
                    i += 1;
                    while i < chars.len() && chars[i] != ')' {
                        i += 1;
                    }
                    if i < chars.len() {
                        i += 1;
                    }
                }
            }
            out.push_str(&text);
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Split display lines into pages of `height` lines each.
fn paginate(lines: &[Line], height: usize) -> Vec<Vec<Line>> {
    if height == 0 || lines.is_empty() {
        return vec![lines.to_vec()];
    }
    lines.chunks(height).map(|c| c.to_vec()).collect()
}

// ── Drawing ────────────────────────────────────────────────────────────────

fn draw_page(
    stdout: &mut impl Write,
    pages: &[Vec<Line>],
    page: usize,
    total: usize,
    cols: u16,
    rows: u16,
) -> io::Result<()> {
    let w = cols as usize;

    queue!(stdout, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;

    // ── Top border + title ─────────────────────────────────────────────────
    draw_box_top(stdout, w)?;
    draw_title_bar(stdout, FILE, w)?;
    draw_box_mid(stdout, w)?;

    // ── Content ────────────────────────────────────────────────────────────
    let page_height = rows as usize - V_OVERHEAD;
    let empty_page: Vec<Line> = vec![];
    let lines = pages.get(page).unwrap_or(&empty_page);

    for i in 0..page_height {
        queue!(
            stdout,
            SetForegroundColor(Color::DarkGrey),
            Print("│"),
            ResetColor,
        )?;

        if let Some(line) = lines.get(i) {
            draw_line(stdout, line, w)?;
        } else {
            // Empty row padding
            queue!(stdout, Print(format!("{:width$}", "", width = w - 2)))?;
        }

        queue!(
            stdout,
            SetForegroundColor(Color::DarkGrey),
            Print("│\r\n"),
            ResetColor,
        )?;
    }

    // ── Bottom divider + status bar ────────────────────────────────────────
    draw_box_mid(stdout, w)?;
    draw_status_bar(stdout, page, total, w)?;
    draw_box_bottom(stdout, w)?;

    stdout.flush()
}

fn draw_line(stdout: &mut impl Write, line: &Line, w: usize) -> io::Result<()> {
    let inner = w - 2; // subtract side borders
    let pad = 2usize;  // left padding inside border

    match line {
        Line::Blank => {
            queue!(stdout, Print(format!("{:width$}", "", width = inner)))?;
        }
        Line::Rule => {
            let rule = format!(
                "{}{}{} ",
                " ".repeat(pad),
                "─".repeat(inner - pad - 1),
                " "
            );
            queue!(
                stdout,
                SetForegroundColor(Color::DarkGrey),
                Print(format!("{:<width$}", rule, width = inner)),
                ResetColor,
            )?;
        }
        Line::H1(text) => {
            let padded = format!("{}{}", " ".repeat(pad), text);
            queue!(
                stdout,
                SetForegroundColor(Color::Cyan),
                SetAttribute(Attribute::Bold),
                Print(format!("{:<width$}", padded, width = inner)),
                SetAttribute(Attribute::Reset),
                ResetColor,
            )?;
        }
        Line::H2(text) => {
            let padded = format!("{}{}", " ".repeat(pad), text);
            queue!(
                stdout,
                SetForegroundColor(Color::Yellow),
                SetAttribute(Attribute::Bold),
                Print(format!("{:<width$}", padded, width = inner)),
                SetAttribute(Attribute::Reset),
                ResetColor,
            )?;
        }
        Line::H3(text) => {
            let padded = format!("{}{}", " ".repeat(pad), text);
            queue!(
                stdout,
                SetForegroundColor(Color::Green),
                Print(format!("{:<width$}", padded, width = inner)),
                ResetColor,
            )?;
        }
        Line::Bold(text) => {
            let padded = format!("{}{}", " ".repeat(pad), text);
            queue!(
                stdout,
                SetAttribute(Attribute::Bold),
                Print(format!("{:<width$}", padded, width = inner)),
                SetAttribute(Attribute::Reset),
            )?;
        }
        Line::Code(text) => {
            let padded = format!("{}{}", " ".repeat(pad), text);
            queue!(
                stdout,
                SetForegroundColor(Color::Magenta),
                Print(format!("{:<width$}", padded, width = inner)),
                ResetColor,
            )?;
        }
        Line::Body(text) => {
            let padded = format!("{}{}", " ".repeat(pad), text);
            queue!(stdout, Print(format!("{:<width$}", padded, width = inner)))?;
        }
    }

    Ok(())
}

// ── Box-drawing helpers ─────────────────────────────────────────────────────

fn draw_box_top(stdout: &mut impl Write, w: usize) -> io::Result<()> {
    let line = format!("╔{}╗\r\n", "═".repeat(w - 2));
    queue!(stdout, SetForegroundColor(Color::DarkGrey), Print(line), ResetColor)
}

fn draw_box_mid(stdout: &mut impl Write, w: usize) -> io::Result<()> {
    let line = format!("╠{}╣\r\n", "═".repeat(w - 2));
    queue!(stdout, SetForegroundColor(Color::DarkGrey), Print(line), ResetColor)
}

fn draw_box_bottom(stdout: &mut impl Write, w: usize) -> io::Result<()> {
    let line = format!("╚{}╝\r\n", "═".repeat(w - 2));
    queue!(stdout, SetForegroundColor(Color::DarkGrey), Print(line), ResetColor)
}

fn draw_title_bar(stdout: &mut impl Write, title: &str, w: usize) -> io::Result<()> {
    let inner = w - 2;
    let title_str = format!("  📖  {}", title);
    queue!(
        stdout,
        SetForegroundColor(Color::DarkGrey),
        Print("║"),
        ResetColor,
        SetForegroundColor(Color::White),
        SetAttribute(Attribute::Bold),
        Print(format!("{:<width$}", title_str, width = inner)),
        SetAttribute(Attribute::Reset),
        ResetColor,
        SetForegroundColor(Color::DarkGrey),
        Print("║\r\n"),
        ResetColor,
    )
}

fn draw_status_bar(stdout: &mut impl Write, page: usize, total: usize, w: usize) -> io::Result<()> {
    let inner = w - 2;
    let left = format!("  Page {} / {}", page + 1, total);
    let right = "  [↑/↓] scroll   [g] start   [G] end   [q] quit  ";
    let gap = inner.saturating_sub(left.len() + right.len());
    let bar = format!("{}{}{}", left, " ".repeat(gap), right);

    queue!(
        stdout,
        SetForegroundColor(Color::DarkGrey),
        Print("║"),
        ResetColor,
        SetForegroundColor(Color::DarkGrey),
        Print(format!("{:<width$}", bar, width = inner)),
        ResetColor,
        SetForegroundColor(Color::DarkGrey),
        Print("║\r\n"),
        ResetColor,
    )
}
