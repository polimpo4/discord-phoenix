//! Renderização do markdown do Discord sem depender de HTML/WebView.
//!
//! Suporta negrito, itálico, sublinhado, riscado, código inline, blocos de
//! código, citações, listas, spoilers, links mascarados e atalhos de emoji.
//! A renderização usa widgets nativos do egui, então links abrem no navegador
//! e spoilers podem ser revelados com um clique.

use eframe::egui::{self, Color32, FontFamily, FontId, RichText, Ui};

use crate::emoji;
use crate::theme;

const BODY_SIZE: f32 = 14.5;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct Style {
    bold: bool,
    italic: bool,
    underline: bool,
    strike: bool,
    code: bool,
}

#[derive(Clone, PartialEq, Eq)]
struct Span {
    text: String,
    style: Style,
    link: Option<String>,
    spoiler: bool,
}

impl Span {
    fn plain(text: impl Into<String>, style: Style) -> Self {
        Self {
            text: text.into(),
            style,
            link: None,
            spoiler: false,
        }
    }
}

pub fn render(ui: &mut Ui, text: &str) {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim_start();

        // Bloco de código: ```linguagem ... ```
        if trimmed.starts_with("```") {
            let language = trimmed.trim_start_matches('`').trim().to_string();
            let mut code = String::new();
            index += 1;
            while index < lines.len() && !lines[index].trim_start().starts_with("```") {
                code.push_str(lines[index]);
                code.push('\n');
                index += 1;
            }
            index += 1; // fecha o bloco
            render_code_block(ui, &code, &language);
            continue;
        }

        // Citação: > texto
        if trimmed.starts_with("> ") || trimmed == ">" {
            let mut quote = Vec::new();
            while index < lines.len() {
                let current = lines[index].trim_start();
                if let Some(rest) = current.strip_prefix("> ") {
                    quote.push(rest.to_string());
                } else if current == ">" {
                    quote.push(String::new());
                } else {
                    break;
                }
                index += 1;
            }
            render_quote(ui, &quote);
            continue;
        }

        if let Some((size, rest)) = heading(trimmed) {
            render_inline_line(ui, rest, size, theme::TEXT);
            ui.add_space(2.0);
            index += 1;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("-# ") {
            render_inline_line(ui, rest, 11.5, theme::MUTED);
            index += 1;
            continue;
        }

        if let Some((prefix, rest)) = list_item(trimmed) {
            ui.horizontal_top(|ui| {
                ui.label(
                    RichText::new(prefix)
                        .size(BODY_SIZE)
                        .color(theme::MUTED),
                );
                render_inline_line(ui, rest, BODY_SIZE, theme::TEXT);
            });
            index += 1;
            continue;
        }

        if trimmed.is_empty() {
            ui.add_space(2.0);
            index += 1;
            continue;
        }

        render_inline_line(ui, line, BODY_SIZE, theme::TEXT);
        index += 1;
    }
}

fn heading(line: &str) -> Option<(f32, &str)> {
    for (marker, size) in [("### ", 16.0), ("## ", 18.0), ("# ", 20.0)] {
        if let Some(rest) = line.strip_prefix(marker) {
            return Some((size, rest));
        }
    }
    None
}

fn list_item(line: &str) -> Option<(&'static str, &str)> {
    for marker in ["- ", "* ", "+ "] {
        if let Some(rest) = line.strip_prefix(marker) {
            return Some(("• ", rest));
        }
    }

    let digits = line.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 {
        let rest = &line[digits..];
        if let Some(text) = rest.strip_prefix(". ") {
            return Some((". ", text));
        }
    }
    None
}

fn render_code_block(ui: &mut Ui, code: &str, language: &str) {
    egui::Frame::none()
        .fill(theme::BG_DARKEST)
        .rounding(6.0)
        .inner_margin(egui::Margin::symmetric(10.0, 8.0))
        .show(ui, |ui| {
            if !language.is_empty() {
                ui.label(
                    RichText::new(language)
                        .size(9.5)
                        .color(theme::MUTED),
                );
            }
            ui.add(
                egui::Label::new(
                    RichText::new(code.trim_end())
                        .font(FontId::monospace(12.5))
                        .color(theme::TEXT),
                )
                .wrap(),
            );
        });
    ui.add_space(4.0);
}

fn render_quote(ui: &mut Ui, lines: &[String]) {
    egui::Frame::none()
        .fill(theme::BG_SIDEBAR)
        .rounding(4.0)
        .inner_margin(egui::Margin {
            left: 10.0,
            right: 8.0,
            top: 4.0,
            bottom: 4.0,
        })
        .show(ui, |ui| {
            for line in lines {
                if line.is_empty() {
                    ui.add_space(2.0);
                } else {
                    render_inline_line(ui, line, BODY_SIZE, theme::MUTED);
                }
            }
        });
    ui.add_space(3.0);
}

fn render_inline_line(ui: &mut Ui, text: &str, size: f32, color: Color32) {
    let mut spans = Vec::new();
    parse_inline(text, Style::default(), false, &mut spans, 0);

    if spans.is_empty() {
        return;
    }

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;

        for span in spans {
            if let Some(ref url) = span.link {
                let label = styled_text(&span, size, theme::LINK);
                let response = ui.hyperlink_to(label, url);
                response.on_hover_text(&*url);
                continue;
            }

            if span.spoiler {
                render_spoiler(ui, &span, size);
                continue;
            }

            if span.style.code {
                ui.add(
                    egui::Label::new(
                        RichText::new(&span.text)
                            .font(FontId::monospace(13.0))
                            .color(Color32::from_rgb(235, 235, 240))
                            .background_color(theme::BG_DARKEST),
                    )
                    .wrap(),
                );
                continue;
            }

            ui.add(egui::Label::new(styled_text(&span, size, color)).wrap());
        }
    });
}

fn styled_text(span: &Span, size: f32, color: Color32) -> RichText {
    let mut text = RichText::new(&span.text).size(size).color(color);

    let family = match (span.style.bold, span.style.italic) {
        (true, true) => Some("bold_italic"),
        (true, false) => Some("bold"),
        (false, true) => Some("italic"),
        (false, false) => None,
    };
    if let Some(family) = family {
        text = text.font(FontId::new(size, FontFamily::Name(family.into())));
    }

    if span.style.underline {
        text = text.underline();
    }
    if span.style.strike {
        text = text.strikethrough();
    }
    text
}

fn render_spoiler(ui: &mut Ui, span: &Span, size: f32) {
    let id = ui.make_persistent_id(("phoenix-spoiler", &span.text));
    let revealed = ui
        .data(|data| data.get_temp::<bool>(id))
        .unwrap_or(false);

    if revealed {
        let mut text = styled_text(span, size, theme::TEXT).background_color(theme::BG_SIDEBAR);
        text = text.color(theme::TEXT);
        ui.add(egui::Label::new(text).wrap());
        return;
    }

    let mask = "█".repeat(span.text.chars().count().clamp(3, 18));
    let button = egui::Button::new(
        RichText::new(mask)
            .size(size * 0.8)
            .color(Color32::from_rgb(92, 96, 104)),
    )
    .frame(false);

    if ui
        .add(button)
        .on_hover_text("Spoiler — clique para revelar")
        .clicked()
    {
        ui.data_mut(|data| data.insert_temp(id, true));
    }
}

fn parse_inline(
    text: &str,
    style: Style,
    spoiler: bool,
    out: &mut Vec<Span>,
    depth: usize,
) {
    if depth > 8 {
        push_text(out, text, style, spoiler);
        return;
    }

    let mut rest = text;
    let mut buffer = String::new();

    while !rest.is_empty() {
        // Escapes: \* mostra o asterisco literal.
        if let Some(stripped) = rest.strip_prefix('\\') {
            if let Some(ch) = stripped.chars().next() {
                if is_marker(ch) {
                    buffer.push(ch);
                    rest = &stripped[ch.len_utf8()..];
                    continue;
                }
            }
        }

        if rest.starts_with('`') {
            let ticks = rest.chars().take_while(|ch| *ch == '`').count();
            let marker = &rest[..ticks];
            if let Some(end) = rest[ticks..].find(marker) {
                flush(&mut buffer, style, spoiler, out);
                let code = &rest[ticks..ticks + end];
                if !code.is_empty() {
                    push_text(
                        out,
                        code,
                        Style {
                            code: true,
                            ..Style::default()
                        },
                        spoiler,
                    );
                }
                rest = &rest[ticks + end + ticks..];
                continue;
            }
        }

        if let Some(inner) = delimited(rest, "***") {
            flush(&mut buffer, style, spoiler, out);
            parse_inline(
                inner.0,
                Style {
                    bold: true,
                    italic: true,
                    ..style
                },
                spoiler,
                out,
                depth + 1,
            );
            rest = inner.1;
            continue;
        }

        if let Some(inner) = delimited(rest, "**") {
            flush(&mut buffer, style, spoiler, out);
            parse_inline(
                inner.0,
                Style {
                    bold: true,
                    ..style
                },
                spoiler,
                out,
                depth + 1,
            );
            rest = inner.1;
            continue;
        }

        if let Some(inner) = delimited(rest, "__") {
            flush(&mut buffer, style, spoiler, out);
            parse_inline(
                inner.0,
                Style {
                    underline: true,
                    ..style
                },
                spoiler,
                out,
                depth + 1,
            );
            rest = inner.1;
            continue;
        }

        if let Some(inner) = delimited(rest, "~~") {
            flush(&mut buffer, style, spoiler, out);
            parse_inline(
                inner.0,
                Style {
                    strike: true,
                    ..style
                },
                spoiler,
                out,
                depth + 1,
            );
            rest = inner.1;
            continue;
        }

        if let Some(inner) = delimited(rest, "||") {
            flush(&mut buffer, style, spoiler, out);
            parse_inline(inner.0, style, true, out, depth + 1);
            rest = inner.1;
            continue;
        }

        if let Some(inner) = delimited(rest, "*") {
            flush(&mut buffer, style, spoiler, out);
            parse_inline(
                inner.0,
                Style {
                    italic: true,
                    ..style
                },
                spoiler,
                out,
                depth + 1,
            );
            rest = inner.1;
            continue;
        }

        if let Some(inner) = delimited(rest, "_") {
            flush(&mut buffer, style, spoiler, out);
            parse_inline(
                inner.0,
                Style {
                    italic: true,
                    ..style
                },
                spoiler,
                out,
                depth + 1,
            );
            rest = inner.1;
            continue;
        }

        if let Some((label, url, consumed)) = link(rest) {
            flush(&mut buffer, style, spoiler, out);
            let mut span = Span::plain(label, style);
            span.link = Some(url.to_string());
            push_span(out, span);
            rest = consumed;
            continue;
        }

        if rest.starts_with(':') {
            if let Some(end) = rest[1..].find(':') {
                let code = &rest[1..1 + end];
                if !code.is_empty()
                    && !code.chars().any(char::is_whitespace)
                    && !code.contains('<')
                    && !code.contains('>')
                {
                    if let Some(emoji) = emoji::find(code) {
                        flush(&mut buffer, style, spoiler, out);
                        push_text(out, emoji.glyph, style, spoiler);
                        rest = &rest[1 + end + 1..];
                        continue;
                    }
                }
            }
        }

        if let Some(ch) = rest.chars().next() {
            buffer.push(ch);
            rest = &rest[ch.len_utf8()..];
        }
    }

    flush(&mut buffer, style, spoiler, out);
}

/// Devolve o conteúdo entre dois marcadores e o restante depois do fechamento.
fn delimited<'a>(text: &'a str, marker: &str) -> Option<(&'a str, &'a str)> {
    let rest = text.strip_prefix(marker)?;
    let end = rest.find(marker)?;
    let inner = &rest[..end];
    if inner.is_empty() {
        return None;
    }
    Some((inner, &rest[end + marker.len()..]))
}

fn link(text: &str) -> Option<(&str, &str, &str)> {
    let rest = text.strip_prefix('[')?;
    let label_end = rest.find("](")?;
    let label = &rest[..label_end];
    let tail = &rest[label_end + 2..];
    let url_end = tail.find(')')?;
    let url = &tail[..url_end];
    if label.is_empty() || url.is_empty() {
        return None;
    }
    Some((label, url, &tail[url_end + 1..]))
}

fn is_marker(ch: char) -> bool {
    matches!(ch, '*' | '_' | '~' | '|' | '`' | '\\' | '[' | ']' | '(' | ')')
}

fn flush(buffer: &mut String, style: Style, spoiler: bool, out: &mut Vec<Span>) {
    if buffer.is_empty() {
        return;
    }
    let text = std::mem::take(buffer);
    push_text(out, &text, style, spoiler);
}

fn push_text(out: &mut Vec<Span>, text: &str, style: Style, spoiler: bool) {
    if text.is_empty() {
        return;
    }
    let mut span = Span::plain(text, style);
    span.spoiler = spoiler;
    push_span(out, span);
}

fn push_span(out: &mut Vec<Span>, span: Span) {
    if let Some(last) = out.last_mut() {
        if last.style == span.style && last.link == span.link && last.spoiler == span.spoiler {
            last.text.push_str(&span.text);
            return;
        }
    }
    out.push(span);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(text: &str) -> Vec<Span> {
        let mut out = Vec::new();
        parse_inline(text, Style::default(), false, &mut out, 0);
        out
    }

    #[test]
    fn parses_bold_italic_underline_and_strike() {
        let parsed = spans("**forte** *leve* __linha__ ~~risco~~");
        assert!(parsed.iter().any(|s| s.style.bold && s.text == "forte"));
        assert!(parsed.iter().any(|s| s.style.italic && s.text == "leve"));
        assert!(parsed.iter().any(|s| s.style.underline && s.text == "linha"));
        assert!(parsed.iter().any(|s| s.style.strike && s.text == "risco"));
    }

    #[test]
    fn parses_code_link_and_spoiler() {
        let parsed = spans("`let x = 1;` [site](https://example.com) ||segredo||");
        assert!(parsed
            .iter()
            .any(|s| s.style.code && s.text == "let x = 1;"));
        assert!(parsed
            .iter()
            .any(|s| s.link.as_deref() == Some("https://example.com")));
        assert!(parsed.iter().any(|s| s.spoiler && s.text == "segredo"));
    }

    #[test]
    fn converts_known_emoji_shortcodes() {
        let parsed = spans("oi :smiley:");
        assert!(parsed.iter().any(|s| s.text.contains('😃')));
    }
}
