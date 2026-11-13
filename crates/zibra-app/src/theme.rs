use eframe::egui::{self, Color32, Rounding, Shadow, Stroke, Vec2};
use zibra_core::{AppSettings, AppTheme, ColorTheme};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct GlassTokens {
    pub primary: Color32,
    pub on_primary: Color32,
    pub primary_container: Color32,
    pub secondary: Color32,
    pub background: Color32,
    pub surface: Color32,
    pub surface_container: Color32,
    pub surface_container_high: Color32,
    pub surface_card: Color32,
    pub surface_glass: Color32,
    pub surface_glass_raised: Color32,
    pub outline: Color32,
    pub outline_variant: Color32,
    pub border_glass: Color32,
    pub border_glow: Color32,
    pub text_primary: Color32,
    pub text_muted: Color32,
    pub is_dark: bool,
    pub is_amoled: bool,
}

impl GlassTokens {
    pub fn from_settings(settings: &AppSettings) -> Self {
        let is_amoled = settings.is_amoled || settings.color_theme == ColorTheme::AmoledVoid;
        let is_dark = settings.theme == AppTheme::Dark || is_amoled;

        let (pr, pg, pb) = if is_dark {
            settings.color_theme.primary_rgb()
        } else {
            settings.color_theme.light_primary_rgb()
        };
        let (sr, sg, sb) = settings.color_theme.secondary_rgb();

        let primary = Color32::from_rgb(pr, pg, pb);
        let secondary = Color32::from_rgb(sr, sg, sb);
        let on_primary = if is_dark {
            Color32::from_rgb(0x0B, 0x0F, 0x19)
        } else {
            Color32::WHITE
        };
        let primary_container = Color32::from_rgba_unmultiplied(pr, pg, pb, if is_dark { 52 } else { 38 });

        let background = if is_amoled {
            Color32::from_rgb(0x00, 0x00, 0x00)
        } else if is_dark {
            let (r, g, b) = settings.color_theme.dark_bg_rgb();
            Color32::from_rgb(r, g, b)
        } else {
            let (r, g, b) = settings.color_theme.light_bg_rgb();
            Color32::from_rgb(r, g, b)
        };

        let surface = if is_amoled {
            Color32::from_rgb(0x07, 0x08, 0x0A)
        } else {
            background
        };

        let surface_container = if is_amoled {
            Color32::from_rgb(0x0E, 0x10, 0x14)
        } else if is_dark {
            let (r, g, b) = settings.color_theme.dark_surface_container_rgb();
            Color32::from_rgb(r, g, b)
        } else {
            let (r, g, b) = settings.color_theme.light_surface_container_rgb();
            Color32::from_rgb(r, g, b)
        };

        let surface_container_high = if is_amoled {
            Color32::from_rgb(0x16, 0x18, 0x1E)
        } else if is_dark {
            let (r, g, b) = settings.color_theme.dark_surface_container_rgb();
            Color32::from_rgb(r.saturating_add(9), g.saturating_add(10), b.saturating_add(14))
        } else {
            let (r, g, b) = settings.color_theme.light_surface_container_rgb();
            Color32::from_rgb(r.saturating_sub(8), g.saturating_sub(8), b.saturating_sub(6))
        };

        let surface_card = if is_amoled {
            Color32::from_rgb(0x11, 0x13, 0x18)
        } else {
            surface_container
        };

        let surface_glass = if is_amoled {
            Color32::from_rgb(0x09, 0x0A, 0x0D)
        } else {
            surface_container
        };

        let surface_glass_raised = surface_container_high;

        let outline = if is_dark {
            Color32::from_rgb(0x8D, 0x91, 0x99)
        } else {
            Color32::from_rgb(0x74, 0x77, 0x7F)
        };

        let outline_variant = if is_dark {
            Color32::from_rgb(0x36, 0x3B, 0x46)
        } else {
            Color32::from_rgb(0xC3, 0xC6, 0xCF)
        };

        let border_glass = if is_dark {
            Color32::from_rgba_unmultiplied(0x52, 0x5A, 0x6E, 110)
        } else {
            Color32::from_rgba_unmultiplied(0x94, 0x9B, 0xAA, 130)
        };

        let border_glow = Color32::from_rgba_unmultiplied(pr, pg, pb, 165);

        let text_primary = if is_dark {
            Color32::from_rgb(0xF0, 0xF2, 0xF8)
        } else {
            Color32::from_rgb(0x16, 0x19, 0x20)
        };

        let text_muted = if is_dark {
            Color32::from_rgb(0x9C, 0xA3, 0xB2)
        } else {
            Color32::from_rgb(0x5E, 0x65, 0x73)
        };

        Self {
            primary,
            on_primary,
            primary_container,
            secondary,
            background,
            surface,
            surface_container,
            surface_container_high,
            surface_card,
            surface_glass,
            surface_glass_raised,
            outline,
            outline_variant,
            border_glass,
            border_glow,
            text_primary,
            text_muted,
            is_dark,
            is_amoled,
        }
    }

    pub fn card_shadow_color(&self, hovered: bool) -> Color32 {
        if hovered {
            let [r, g, b, _] = self.primary.to_array();
            Color32::from_rgba_unmultiplied(r, g, b, 45)
        } else {
            Color32::from_rgba_unmultiplied(0, 0, 0, if self.is_dark { 95 } else { 28 })
        }
    }
}

pub fn apply_kotatsu_visuals(ctx: &egui::Context, tokens: &GlassTokens) {
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(10.0, 8.0);
    style.spacing.button_padding = Vec2::new(12.0, 7.0);
    style.spacing.window_margin = egui::Margin::same(14.0);

    let mut visuals = if tokens.is_dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    visuals.panel_fill = tokens.background;
    visuals.window_fill = tokens.surface_container_high;
    visuals.faint_bg_color = tokens.surface_container;
    visuals.extreme_bg_color = tokens.surface_card;
    visuals.window_rounding = Rounding::same(14.0);
    visuals.window_stroke = Stroke::new(1.0_f32, tokens.border_glass);
    visuals.override_text_color = None;
    visuals.popup_shadow = Shadow::NONE;

    visuals.selection.bg_fill = tokens.primary_container;
    visuals.selection.stroke = Stroke::new(1.5_f32, tokens.primary);

    visuals.widgets.noninteractive.bg_fill = tokens.surface_card;
    visuals.widgets.noninteractive.weak_bg_fill = tokens.surface_container;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, tokens.border_glass);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, tokens.text_primary);
    visuals.widgets.noninteractive.rounding = Rounding::same(10.0);

    visuals.widgets.inactive.bg_fill = tokens.surface_container_high;
    visuals.widgets.inactive.weak_bg_fill = tokens.surface_container;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, tokens.border_glass);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, tokens.text_primary);
    visuals.widgets.inactive.rounding = Rounding::same(10.0);

    visuals.widgets.hovered.bg_fill = tokens.primary_container;
    visuals.widgets.hovered.weak_bg_fill = tokens.surface_container_high;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.5_f32, tokens.border_glow);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.5_f32, tokens.primary);
    visuals.widgets.hovered.rounding = Rounding::same(10.0);

    visuals.widgets.active.bg_fill = tokens.primary;
    visuals.widgets.active.weak_bg_fill = tokens.primary_container;
    visuals.widgets.active.bg_stroke = Stroke::new(1.5_f32, tokens.primary);
    visuals.widgets.active.fg_stroke = Stroke::new(1.5_f32, tokens.on_primary);
    visuals.widgets.active.rounding = Rounding::same(10.0);

    style.visuals = visuals;
    ctx.set_style(style);
}
