use eframe::egui::{self, Color32, FontId, TextStyle};

pub const BG_DARKEST: Color32 = Color32::from_rgb(30, 31, 34);
pub const BG_SIDEBAR: Color32 = Color32::from_rgb(43, 45, 49);
pub const BG_CHAT: Color32 = Color32::from_rgb(49, 51, 56);
pub const BG_INPUT: Color32 = Color32::from_rgb(56, 58, 64);
pub const BG_HOVER: Color32 = Color32::from_rgb(57, 60, 66);
pub const BG_SELECTED: Color32 = Color32::from_rgb(66, 70, 78);
pub const TEXT: Color32 = Color32::from_rgb(219, 222, 225);
pub const MUTED: Color32 = Color32::from_rgb(148, 155, 164);
pub const ACCENT: Color32 = Color32::from_rgb(88, 101, 242);
pub const GREEN: Color32 = Color32::from_rgb(35, 165, 90);
pub const YELLOW: Color32 = Color32::from_rgb(240, 178, 50);
pub const RED: Color32 = Color32::from_rgb(242, 63, 66);
pub const LINK: Color32 = Color32::from_rgb(0, 168, 252);

const NAME_COLORS: [Color32; 10] = [
    Color32::from_rgb(88, 101, 242),
    Color32::from_rgb(59, 165, 92),
    Color32::from_rgb(242, 101, 34),
    Color32::from_rgb(235, 69, 158),
    Color32::from_rgb(254, 231, 92),
    Color32::from_rgb(0, 168, 252),
    Color32::from_rgb(155, 89, 182),
    Color32::from_rgb(26, 188, 156),
    Color32::from_rgb(230, 126, 34),
    Color32::from_rgb(231, 76, 60),
];

const AVATAR_COLORS: [Color32; 12] = [
    Color32::from_rgb(88, 101, 242),
    Color32::from_rgb(59, 165, 92),
    Color32::from_rgb(242, 101, 34),
    Color32::from_rgb(235, 69, 158),
    Color32::from_rgb(250, 166, 26),
    Color32::from_rgb(0, 168, 252),
    Color32::from_rgb(155, 89, 182),
    Color32::from_rgb(26, 188, 156),
    Color32::from_rgb(230, 126, 34),
    Color32::from_rgb(231, 76, 60),
    Color32::from_rgb(52, 152, 219),
    Color32::from_rgb(46, 204, 113),
];

pub fn hash_id(id: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in id.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub fn color_for(id: &str) -> Color32 {
    AVATAR_COLORS[(hash_id(id) as usize) % AVATAR_COLORS.len()]
}

pub fn name_color(id: &str) -> Color32 {
    NAME_COLORS[(hash_id(id) as usize) % NAME_COLORS.len()]
}

pub fn apply(ctx: &egui::Context) {
    install_system_fonts(ctx);

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG_CHAT;
    visuals.window_fill = BG_SIDEBAR;
    visuals.extreme_bg_color = BG_DARKEST;
    visuals.faint_bg_color = BG_INPUT;
    visuals.override_text_color = Some(TEXT);
    visuals.selection.bg_fill = ACCENT;
    visuals.selection.stroke.color = Color32::WHITE;
    visuals.hyperlink_color = LINK;
    visuals.widgets.noninteractive.bg_fill = BG_SIDEBAR;
    visuals.widgets.noninteractive.fg_stroke.color = TEXT;
    visuals.widgets.inactive.bg_fill = BG_INPUT;
    visuals.widgets.inactive.fg_stroke.color = TEXT;
    visuals.widgets.hovered.bg_fill = BG_HOVER;
    visuals.widgets.hovered.fg_stroke.color = Color32::WHITE;
    visuals.widgets.active.bg_fill = ACCENT;
    visuals.widgets.active.fg_stroke.color = Color32::WHITE;
    visuals.widgets.open.bg_fill = BG_HOVER;
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.spacing.scroll.bar_width = 6.0;
    style.interaction.selectable_labels = true;
    style.text_styles.insert(TextStyle::Body, FontId::proportional(15.0));
    style.text_styles.insert(TextStyle::Button, FontId::proportional(14.0));
    style.text_styles.insert(TextStyle::Small, FontId::proportional(12.0));
    style.text_styles.insert(TextStyle::Heading, FontId::proportional(22.0));
    style.text_styles.insert(TextStyle::Monospace, FontId::monospace(13.0));
    ctx.set_style(style);
}

pub fn accent_button(text: impl Into<egui::WidgetText>) -> egui::Button<'static> {
    egui::Button::new(text)
        .fill(ACCENT)
        .stroke(egui::Stroke::NONE)
        .rounding(8.0)
}

/// Carrega fontes do próprio Windows como fallback. O egui já traz
/// NotoEmoji para emojis e Ubuntu-Light para o latim básico, mas caracteres
/// de Alt Code (setas, caixas, estrelas, símbolos) e latim estendido dependem
/// de fontes do sistema. Segoe UI cobre o texto e Segoe UI Symbol cobre os
/// símbolos usados em códigos e mensagens.
fn install_system_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    for (name, file) in [
        ("Segoe UI", r"C:\Windows\Fonts\segoeui.ttf"),
        ("Segoe UI Bold", r"C:\Windows\Fonts\segoeuib.ttf"),
        ("Segoe UI Italic", r"C:\Windows\Fonts\segoeuii.ttf"),
        ("Segoe UI Bold Italic", r"C:\Windows\Fonts\segoeuiz.ttf"),
        ("Segoe UI Symbol", r"C:\Windows\Fonts\seguisym.ttf"),
    ] {
        let Ok(bytes) = std::fs::read(file) else {
            continue;
        };
        fonts
            .font_data
            .insert(name.to_owned(), egui::FontData::from_owned(bytes));

        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            if let Some(list) = fonts.families.get_mut(&family) {
                list.push(name.to_owned());
            }
        }
    }

    // Famílias próprias para o markdown. O egui não sintetiza negrito/itálico,
    // então usamos os arquivos de fonte do Windows e mantemos todas as fontes
    // de fallback (incluindo o NotoEmoji) depois deles.
    let proportional = fonts
        .families
        .get(&egui::FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();

    for (family, first) in [
        ("bold", "Segoe UI Bold"),
        ("italic", "Segoe UI Italic"),
        ("bold_italic", "Segoe UI Bold Italic"),
    ] {
        if !fonts.font_data.contains_key(first) {
            continue;
        }
        let mut list = proportional.clone();
        list.insert(0, first.to_owned());
        fonts
            .families
            .insert(egui::FontFamily::Name(family.into()), list);
    }

    ctx.set_fonts(fonts);
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn system_fonts_cover_accents_alt_codes_and_emoji() {
        let ctx = egui::Context::default();
        apply(&ctx);

        // Força a criação do atlas de fontes antes de consultar os glifos.
        let mut input = egui::RawInput::default();
        input.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(640.0, 480.0),
        ));
        let _ = ctx.run(input, |_| {});

        let sample = "áéíóú ç ãõ âê ô à ü → ← ↑ ↓ ★ ☺ ♥ ⚡ ⚙ ✓ │ ─ █ ▓ 😀 🎉 🚀 ❤ 👍🏽 🔥 ✅ 👀 🤖 ✨";
        let font_id = egui::FontId::proportional(15.0);
        let missing: Vec<char> = sample
            .chars()
            .filter(|c| !c.is_whitespace() && !ctx.fonts(|fonts| fonts.has_glyph(&font_id, *c)))
            .collect();

        assert!(missing.is_empty(), "glifos ausentes: {missing:?}");
    }
}
