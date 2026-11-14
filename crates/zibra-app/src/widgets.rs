use crate::theme::GlassTokens;
use eframe::egui::{
    self, Align2, Color32, ColorImage, FontId, Pos2, Rect, RichText, Rounding, Sense, Stroke,
    TextureFilter, TextureHandle, TextureOptions, TextureWrapMode, Vec2,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;
use zibra_core::{Manga, ReadingProgress};
use zibra_reader::{CoverImageCache, CoverStatus};

pub const HIGH_QUALITY_TEXTURE: TextureOptions = TextureOptions {
    magnification: TextureFilter::Linear,
    minification: TextureFilter::Linear,
    wrap_mode: TextureWrapMode::ClampToEdge,
    mipmap_mode: Some(TextureFilter::Linear),
};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeIcon {
    History,
    Favorites,
    Explore,
    Feed,
    Downloads,
    Settings,
    Fullscreen,
    Amoled,
    Back,
    Close,
    Search,
    ZoomIn,
    ZoomOut,
    Refresh,
}

pub fn paint_native_icon(
    painter: &egui::Painter,
    rect: Rect,
    icon: NativeIcon,
    color: Color32,
    stroke_w: f32,
) {
    let center = rect.center();
    let r = (rect.width().min(rect.height()) * 0.38).clamp(3.0, 18.0);
    let stroke = Stroke::new(stroke_w, color);

    match icon {
        NativeIcon::History => {
            painter.circle_stroke(center, r * 0.85, stroke);
            painter.circle_filled(center, stroke_w * 0.9, color);
            painter.line_segment([center, center + Vec2::new(r * 0.38, -r * 0.28)], stroke);
            painter.line_segment([center, center + Vec2::new(0.0, -r * 0.55)], stroke);
        }
        NativeIcon::Favorites => {
            let n_points = 32;
            let mut points = Vec::with_capacity(n_points + 1);
            let scale = r / 16.5;
            for i in 0..=n_points {
                let t = (i as f32 / n_points as f32) * std::f32::consts::TAU;
                let sint = t.sin();
                let x = 16.0 * sint * sint * sint;
                let y = -(13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos());
                points.push(center + Vec2::new(x * scale, (y + 1.8) * scale));
            }
            painter.add(egui::epaint::PathShape::line(points, stroke));
        }
        NativeIcon::Explore => {
            painter.circle_stroke(center, r * 0.90, stroke);
            painter.line_segment([center + Vec2::new(0.0, -r * 0.90), center + Vec2::new(0.0, -r * 0.72)], stroke);
            painter.line_segment([center + Vec2::new(0.0, r * 0.72), center + Vec2::new(0.0, r * 0.90)], stroke);
            painter.line_segment([center + Vec2::new(-r * 0.90, 0.0), center + Vec2::new(-r * 0.72, 0.0)], stroke);
            painter.line_segment([center + Vec2::new(r * 0.72, 0.0), center + Vec2::new(r * 0.90, 0.0)], stroke);
            let tip_n = center + Vec2::new(r * 0.55, -r * 0.55);
            let tip_s = center + Vec2::new(-r * 0.55, r * 0.55);
            let side1 = center + Vec2::new(-r * 0.16, -r * 0.16);
            let side2 = center + Vec2::new(r * 0.16, r * 0.16);
            painter.add(egui::epaint::PathShape::convex_polygon(
                vec![side1, tip_n, side2],
                color,
                Stroke::NONE,
            ));
            painter.add(egui::epaint::PathShape::convex_polygon(
                vec![side1, tip_s, side2],
                Color32::TRANSPARENT,
                stroke,
            ));
            painter.circle_filled(center, stroke_w * 0.8, color);
        }
        NativeIcon::Feed => {
            let origin = center + Vec2::new(-r * 0.65, r * 0.65);
            painter.circle_filled(origin, stroke_w * 1.3, color);
            let n_arc = 8;
            let mut arc1 = Vec::with_capacity(n_arc + 1);
            let r1 = r * 0.65;
            for i in 0..=n_arc {
                let a = (i as f32 / n_arc as f32) * (std::f32::consts::FRAC_PI_2);
                arc1.push(origin + Vec2::new(r1 * a.cos(), -r1 * a.sin()));
            }
            painter.add(egui::epaint::PathShape::line(arc1, stroke));
            let mut arc2 = Vec::with_capacity(n_arc + 1);
            let r2 = r * 1.15;
            for i in 0..=n_arc {
                let a = (i as f32 / n_arc as f32) * (std::f32::consts::FRAC_PI_2);
                arc2.push(origin + Vec2::new(r2 * a.cos(), -r2 * a.sin()));
            }
            painter.add(egui::epaint::PathShape::line(arc2, stroke));
        }
        NativeIcon::Downloads => {
            let tray_w = r * 0.82;
            let tray_y = center.y + r * 0.65;
            let tray_h = r * 0.35;
            painter.line_segment([Pos2::new(center.x - tray_w, tray_y - tray_h), Pos2::new(center.x - tray_w, tray_y)], stroke);
            painter.line_segment([Pos2::new(center.x - tray_w, tray_y), Pos2::new(center.x + tray_w, tray_y)], stroke);
            painter.line_segment([Pos2::new(center.x + tray_w, tray_y), Pos2::new(center.x + tray_w, tray_y - tray_h)], stroke);
            let arrow_tip = center + Vec2::new(0.0, r * 0.32);
            painter.line_segment([center + Vec2::new(0.0, -r * 0.75), arrow_tip], stroke);
            let wing_w = r * 0.40;
            let wing_h = r * 0.40;
            painter.line_segment([Pos2::new(center.x - wing_w, arrow_tip.y - wing_h), arrow_tip], stroke);
            painter.line_segment([Pos2::new(center.x + wing_w, arrow_tip.y - wing_h), arrow_tip], stroke);
        }
        NativeIcon::Settings => {
            painter.circle_stroke(center, r * 0.35, stroke);
            let n_teeth = 6;
            for i in 0..n_teeth {
                let angle = (i as f32) * std::f32::consts::TAU / (n_teeth as f32);
                let half_span = std::f32::consts::PI / (n_teeth as f32 * 2.8);
                let r_inner = r * 0.58;
                let r_outer = r * 0.88;
                let a1 = angle - half_span;
                let a2 = angle + half_span;
                let p1 = center + Vec2::new(r_inner * a1.cos(), r_inner * a1.sin());
                let p2 = center + Vec2::new(r_outer * a1.cos(), r_outer * a1.sin());
                let p3 = center + Vec2::new(r_outer * a2.cos(), r_outer * a2.sin());
                let p4 = center + Vec2::new(r_inner * a2.cos(), r_inner * a2.sin());
                painter.line_segment([p1, p2], stroke);
                painter.line_segment([p2, p3], stroke);
                painter.line_segment([p3, p4], stroke);
            }
            painter.circle_stroke(center, r * 0.58, stroke);
        }
        NativeIcon::Fullscreen => {
            let d = r * 0.78;
            let arm = r * 0.36;
            painter.line_segment([center + Vec2::new(-d, -d + arm), center + Vec2::new(-d, -d)], stroke);
            painter.line_segment([center + Vec2::new(-d, -d), center + Vec2::new(-d + arm, -d)], stroke);
            painter.line_segment([center + Vec2::new(d - arm, -d), center + Vec2::new(d, -d)], stroke);
            painter.line_segment([center + Vec2::new(d, -d), center + Vec2::new(d, -d + arm)], stroke);
            painter.line_segment([center + Vec2::new(-d, d - arm), center + Vec2::new(-d, d)], stroke);
            painter.line_segment([center + Vec2::new(-d, d), center + Vec2::new(-d + arm, d)], stroke);
            painter.line_segment([center + Vec2::new(d - arm, d), center + Vec2::new(d, d)], stroke);
            painter.line_segment([center + Vec2::new(d, d), center + Vec2::new(d, d - arm)], stroke);
        }
        NativeIcon::Amoled => {
            let n = 16;
            let mut moon = Vec::with_capacity(n * 2 + 1);
            let r_outer = r * 0.85;
            for i in 0..=n {
                let a = -std::f32::consts::FRAC_PI_2 + (i as f32 / n as f32) * std::f32::consts::PI;
                moon.push(center + Vec2::new(r_outer * a.cos(), r_outer * a.sin()));
            }
            let inner_c = center + Vec2::new(r * 0.32, 0.0);
            let r_inner = r * 0.72;
            for i in (0..=n).rev() {
                let a = -std::f32::consts::FRAC_PI_2 + (i as f32 / n as f32) * std::f32::consts::PI;
                moon.push(inner_c + Vec2::new(r_inner * a.cos(), r_inner * a.sin()));
            }
            painter.add(egui::epaint::PathShape::convex_polygon(
                moon,
                color,
                Stroke::NONE,
            ));
        }
        NativeIcon::Back => {
            let arm_x = r * 0.32;
            let arm_y = r * 0.58;
            let tip = center + Vec2::new(-arm_x, 0.0);
            painter.line_segment([center + Vec2::new(arm_x, -arm_y), tip], stroke);
            painter.line_segment([tip, center + Vec2::new(arm_x, arm_y)], stroke);
        }
        NativeIcon::Close => {
            let d = r * 0.55;
            painter.line_segment([center + Vec2::new(-d, -d), center + Vec2::new(d, d)], stroke);
            painter.line_segment([center + Vec2::new(d, -d), center + Vec2::new(-d, d)], stroke);
        }
        NativeIcon::Search => {
            let r_lens = r * 0.50;
            let c_lens = center + Vec2::new(-r * 0.22, -r * 0.22);
            painter.circle_stroke(c_lens, r_lens, stroke);
            let handle_start = c_lens + Vec2::new(r_lens * 0.707, r_lens * 0.707);
            let handle_end = center + Vec2::new(r * 0.80, r * 0.80);
            painter.line_segment([handle_start, handle_end], stroke);
        }
        NativeIcon::ZoomIn => {
            let r_lens = r * 0.50;
            let c_lens = center + Vec2::new(-r * 0.22, -r * 0.22);
            painter.circle_stroke(c_lens, r_lens, stroke);
            let handle_start = c_lens + Vec2::new(r_lens * 0.707, r_lens * 0.707);
            let handle_end = center + Vec2::new(r * 0.80, r * 0.80);
            painter.line_segment([handle_start, handle_end], stroke);
            let arm = r_lens * 0.52;
            painter.line_segment([c_lens + Vec2::new(-arm, 0.0), c_lens + Vec2::new(arm, 0.0)], stroke);
            painter.line_segment([c_lens + Vec2::new(0.0, -arm), c_lens + Vec2::new(0.0, arm)], stroke);
        }
        NativeIcon::ZoomOut => {
            let r_lens = r * 0.50;
            let c_lens = center + Vec2::new(-r * 0.22, -r * 0.22);
            painter.circle_stroke(c_lens, r_lens, stroke);
            let handle_start = c_lens + Vec2::new(r_lens * 0.707, r_lens * 0.707);
            let handle_end = center + Vec2::new(r * 0.80, r * 0.80);
            painter.line_segment([handle_start, handle_end], stroke);
            let arm = r_lens * 0.52;
            painter.line_segment([c_lens + Vec2::new(-arm, 0.0), c_lens + Vec2::new(arm, 0.0)], stroke);
        }
        NativeIcon::Refresh => {
            let arc_r = r * 0.68;
            let n_pts = 16;
            let mut arc = Vec::with_capacity(n_pts + 1);
            let start_angle = -std::f32::consts::FRAC_PI_2;
            let end_angle = std::f32::consts::PI;
            for i in 0..=n_pts {
                let a = start_angle + (i as f32 / n_pts as f32) * (end_angle - start_angle);
                arc.push(center + Vec2::new(arc_r * a.cos(), arc_r * a.sin()));
            }
            painter.add(egui::epaint::PathShape::line(arc, stroke));
            let tip = center + Vec2::new(0.0, -arc_r);
            painter.line_segment([tip + Vec2::new(-r * 0.25, -r * 0.25), tip], stroke);
            painter.line_segment([tip + Vec2::new(-r * 0.25, r * 0.25), tip], stroke);
        }
    }
}

pub fn z_icon_button(
    ui: &mut egui::Ui,
    tokens: &GlassTokens,
    icon: NativeIcon,
    size: Vec2,
    selected: bool,
    tooltip: &str,
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let hovered = resp.hovered();
    let hover_t = ui.ctx().animate_bool_responsive(resp.id, hovered || selected);

    let bg = if selected {
        tokens.primary_container
    } else {
        Color32::from_rgba_unmultiplied(
            tokens.surface_container_high.r(),
            tokens.surface_container_high.g(),
            tokens.surface_container_high.b(),
            (tokens.surface_container_high.a() as f32 * hover_t) as u8,
        )
    };

    let border = if selected {
        Stroke::new(1.5_f32, tokens.primary)
    } else if hover_t > 0.05 {
        Stroke::new(1.0_f32, tokens.border_glass.gamma_multiply(hover_t))
    } else {
        Stroke::NONE
    };

    let fg = if selected {
        tokens.primary
    } else {
        tokens.text_muted.lerp_to_gamma(tokens.text_primary, hover_t)
    };

    let painter = ui.painter_at(rect);
    if bg != Color32::TRANSPARENT {
        painter.rect_filled(rect, Rounding::same(8.0), bg);
    }
    if border != Stroke::NONE {
        painter.rect_stroke(rect, Rounding::same(8.0), border);
    }

    paint_native_icon(&painter, rect, icon, fg, 1.6_f32);

    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    if !tooltip.is_empty() {
        resp.on_hover_text(tooltip).clicked()
    } else {
        resp.clicked()
    }
}

/// Cyber-Glass Search Bar matching ZERAVRIX ZSearchBar
pub fn z_search_bar(
    ui: &mut egui::Ui,
    tokens: &GlassTokens,
    id_salt: &str,
    query: &mut String,
    hint: &str,
    width: f32,
) -> bool {
    let mut submitted = false;
    egui::Frame::none()
        .fill(tokens.surface_card)
        .stroke(Stroke::new(1.0_f32, tokens.border_glass))
        .rounding(Rounding::same(12.0))
        .inner_margin(egui::Margin::symmetric(12.0, 6.0))
        .show(ui, |ui| {
            ui.set_width((width - 26.0).max(120.0));
            ui.horizontal(|ui| {
                let (icon_rect, _) = ui.allocate_exact_size(Vec2::new(18.0, 18.0), Sense::hover());
                paint_native_icon(
                    &ui.painter_at(icon_rect),
                    icon_rect,
                    NativeIcon::Search,
                    tokens.primary,
                    1.6_f32,
                );
                let edit = egui::TextEdit::singleline(query)
                    .id(egui::Id::new(id_salt))
                    .hint_text(RichText::new(hint).color(tokens.text_muted).size(13.0))
                    .frame(false)
                    .desired_width((ui.available_width() - 32.0).max(80.0))
                    .text_color(tokens.text_primary);
                let resp = ui.add(edit);
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    submitted = true;
                }
                if !query.is_empty() {
                    let clear_btn = egui::Button::new(
                        RichText::new("✕").size(11.0).color(tokens.text_muted),
                    )
                    .fill(tokens.surface_container_high)
                    .rounding(Rounding::same(12.0))
                    .min_size(Vec2::new(22.0, 22.0));
                    if ui.add(clear_btn).on_hover_text("Clear search").clicked() {
                        query.clear();
                        submitted = true;
                    }
                }
            });
        });
    submitted
}

/// Animated Cyber-Glass Filter Chip matching ZERAVRIX ZChip
pub fn z_chip(
    ui: &mut egui::Ui,
    tokens: &GlassTokens,
    label: &str,
    icon: Option<&str>,
    selected: bool,
) -> bool {
    let text = match icon {
        Some(ic) => format!("{}  {}", ic, label),
        None => label.to_string(),
    };
    let fg = if selected {
        tokens.primary
    } else {
        tokens.text_muted
    };
    let bg = if selected {
        tokens.primary_container
    } else {
        tokens.surface_card
    };
    let border = if selected {
        Stroke::new(1.5_f32, tokens.primary)
    } else {
        Stroke::new(1.0_f32, tokens.border_glass)
    };

    let btn = egui::Button::new(
        RichText::new(text)
            .size(12.5)
            .color(fg)
            .strong(),
    )
    .fill(bg)
    .stroke(border)
    .rounding(Rounding::same(10.0))
    .min_size(Vec2::new(0.0, 32.0));

    ui.add(btn).clicked()
}

/// Async Cover & Icon Renderer backed by CoverImageCache + GPU TextureHandle cache
pub fn draw_remote_image(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    rt: &Arc<Runtime>,
    cover_cache: &CoverImageCache,
    cover_textures: &mut HashMap<String, TextureHandle>,
    tokens: &GlassTokens,
    url: Option<&str>,
    fallback_title: &str,
    size: Vec2,
    rounding: f32,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    if !ui.is_rect_visible(rect) {
        return resp;
    }

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, Rounding::same(rounding), tokens.surface_container_high);

    if let Some(u) = url.map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(tex) = cover_textures.get(u) {
            painter.image(
                tex.id(),
                rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
            return resp;
        }

        match cover_cache.get_status(u) {
            CoverStatus::Ready(decoded) => {
                let color_img = ColorImage::from_rgba_unmultiplied(
                    [decoded.width as usize, decoded.height as usize],
                    &decoded.rgba_bytes,
                );
                let handle = ctx.load_texture(
                    format!("cov_{}", u),
                    color_img,
                    HIGH_QUALITY_TEXTURE,
                );
                painter.image(
                    handle.id(),
                    rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
                cover_textures.insert(u.to_string(), handle);
                return resp;
            }
            CoverStatus::Loading => {
                draw_cover_placeholder(&painter, rect, tokens, fallback_title, true, rounding);
                return resp;
            }
            CoverStatus::Idle => {
                let cache_clone = cover_cache.clone();
                let u_str = u.to_string();
                let egui_ctx = ctx.clone();
                let max_dim = (size.x.max(size.y) * 3.0).clamp(600.0, 1920.0) as u32;
                rt.spawn(async move {
                    let _ = cache_clone.fetch_and_decode(&u_str, max_dim).await;
                    egui_ctx.request_repaint();
                });
                draw_cover_placeholder(&painter, rect, tokens, fallback_title, true, rounding);
                return resp;
            }
            CoverStatus::Failed => {
                draw_cover_placeholder(&painter, rect, tokens, fallback_title, false, rounding);
                return resp;
            }
        }
    }

    draw_cover_placeholder(&painter, rect, tokens, fallback_title, false, rounding);
    resp
}

fn draw_cover_placeholder(
    painter: &egui::Painter,
    rect: Rect,
    tokens: &GlassTokens,
    title: &str,
    loading: bool,
    rounding: f32,
) {
    let mut hash: u32 = 5381;
    for b in title.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(b as u32);
    }
    let [pr, pg, pb, _] = tokens.primary.to_array();
    let tint = Color32::from_rgba_unmultiplied(
        pr.saturating_sub(((hash & 0x1F) as u8) / 2),
        pg.saturating_sub((((hash >> 5) & 0x1F) as u8) / 2),
        pb,
        28,
    );
    painter.rect_filled(rect, Rounding::same(rounding), tint);

    let initials: String = title
        .split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase();
    let badge_text = if initials.is_empty() {
        "📚".to_string()
    } else {
        initials
    };

    let center = rect.center();
    let circle_r = (rect.width().min(rect.height()) * 0.24).clamp(12.0, 28.0);
    painter.circle_filled(center, circle_r, tokens.primary_container);
    painter.circle_stroke(center, circle_r, Stroke::new(1.0_f32, tokens.border_glow));
    painter.text(
        center,
        Align2::CENTER_CENTER,
        badge_text,
        FontId::proportional((circle_r * 0.78).clamp(10.0, 18.0)),
        tokens.primary,
    );

    if loading && rect.height() > 80.0 {
        painter.text(
            Pos2::new(center.x, rect.max.y - 18.0),
            Align2::CENTER_CENTER,
            "Loading cover…",
            FontId::proportional(10.0),
            tokens.text_muted,
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardAction {
    None,
    Open,
    RemoveFavorite,
    RemoveHistory,
}

/// Rich 13:18 Manga Cover Card matching ZERAVRIX ModernMangaCard & ShelfCard
pub fn manga_cover_card(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    rt: &Arc<Runtime>,
    cover_cache: &CoverImageCache,
    cover_textures: &mut HashMap<String, TextureHandle>,
    tokens: &GlassTokens,
    manga: &Manga,
    progress_percent: Option<u32>,
    _show_source_badge: bool,
) -> CardAction {
    let mut action = CardAction::None;
    let card_w = 148.0;
    let cover_h = 204.0;
    let total_h = cover_h + 44.0;

    let (outer_rect, resp) = ui.allocate_exact_size(Vec2::new(card_w, total_h), Sense::click());
    let hovered = resp.hovered();
    let hover_t = ctx.animate_bool_responsive(resp.id.with("card_hover"), hovered);

    if resp.clicked() {
        action = CardAction::Open;
    }

    resp.context_menu(|ui| {
        ui.label(RichText::new(&manga.title).strong().color(tokens.primary));
        ui.separator();
        if ui.button("📖  Open Details & Chapters").clicked() {
            action = CardAction::Open;
            ui.close_menu();
        }
        if ui.button("💔  Remove from Favorites").clicked() {
            action = CardAction::RemoveFavorite;
            ui.close_menu();
        }
        if ui.button("🗑  Remove from History").clicked() {
            action = CardAction::RemoveHistory;
            ui.close_menu();
        }
    });

    if !ui.is_rect_visible(outer_rect) {
        return action;
    }

    let cover_rect = Rect::from_min_size(outer_rect.min, Vec2::new(card_w, cover_h));

    // Dynamic fluid elevation shadow behind cover
    let shadow_y = egui::lerp(2.0..=5.0, hover_t);
    let shadow_expand = egui::lerp(0.5..=2.5, hover_t);
    let shadow_rect = cover_rect
        .translate(Vec2::new(0.0, shadow_y))
        .expand(shadow_expand);

    let base_shadow = tokens.card_shadow_color(false);
    let hover_shadow = tokens.card_shadow_color(true);
    let shadow_color = Color32::from_rgba_premultiplied(
        egui::lerp(base_shadow.r() as f32..=hover_shadow.r() as f32, hover_t) as u8,
        egui::lerp(base_shadow.g() as f32..=hover_shadow.g() as f32, hover_t) as u8,
        egui::lerp(base_shadow.b() as f32..=hover_shadow.b() as f32, hover_t) as u8,
        egui::lerp(base_shadow.a() as f32..=hover_shadow.a() as f32, hover_t) as u8,
    );
    ui.painter().rect_filled(shadow_rect, Rounding::same(13.0), shadow_color);

    // Render cover image inside cover_rect (pristine artwork, no badges or text clutter)
    let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(cover_rect));
    if draw_remote_image(
        &mut child_ui,
        ctx,
        rt,
        cover_cache,
        cover_textures,
        tokens,
        manga.cover_url.as_deref(),
        &manga.title,
        Vec2::new(card_w, cover_h),
        12.0,
    )
    .clicked()
    {
        action = CardAction::Open;
    }

    let painter = ui.painter();

    // Sleek minimal progress bar along the bottom of the cover (no numbers or bulky circles)
    if let Some(pct) = progress_percent.filter(|&p| p > 0) {
        let bar_w = (card_w * (pct as f32 / 100.0).clamp(0.0, 1.0)).max(6.0);
        let bar_rect = Rect::from_min_size(
            Pos2::new(cover_rect.min.x, cover_rect.max.y - 3.5),
            Vec2::new(bar_w, 3.5),
        );
        painter.rect_filled(
            bar_rect,
            Rounding {
                nw: 0.0,
                ne: 2.0,
                sw: 12.0,
                se: if pct >= 96 { 12.0 } else { 0.0 },
            },
            tokens.primary,
        );
    }

    // Cover Border: smooth animated transition between glass border and glowing primary
    let border_width = egui::lerp(1.0..=2.0, hover_t);
    let border_color = Color32::from_rgba_premultiplied(
        egui::lerp(tokens.border_glass.r() as f32..=tokens.primary.r() as f32, hover_t) as u8,
        egui::lerp(tokens.border_glass.g() as f32..=tokens.primary.g() as f32, hover_t) as u8,
        egui::lerp(tokens.border_glass.b() as f32..=tokens.primary.b() as f32, hover_t) as u8,
        egui::lerp(tokens.border_glass.a() as f32..=tokens.primary.a() as f32, hover_t) as u8,
    );
    painter.rect_stroke(cover_rect, Rounding::same(12.0), Stroke::new(border_width, border_color));

    // 2-line Title below cover with animated primary color accent on hover
    let title_color = Color32::from_rgba_premultiplied(
        egui::lerp(tokens.text_primary.r() as f32..=tokens.primary.r() as f32, hover_t) as u8,
        egui::lerp(tokens.text_primary.g() as f32..=tokens.primary.g() as f32, hover_t) as u8,
        egui::lerp(tokens.text_primary.b() as f32..=tokens.primary.b() as f32, hover_t) as u8,
        egui::lerp(tokens.text_primary.a() as f32..=tokens.primary.a() as f32, hover_t) as u8,
    );
    let clean_title = truncate_str(&manga.title, 34);
    let title_pos = Pos2::new(outer_rect.min.x + 2.0, cover_rect.max.y + 6.0);
    let galley = painter.layout(
        clean_title,
        FontId::proportional(12.5),
        title_color,
        card_w - 4.0,
    );
    painter.galley(title_pos, galley, title_color);

    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    action
}

/// Floating Bottom-Right "Continue Reading" Pill Button (matching ZERAVRIX Main.kt & screenshot)
pub fn floating_continue_button(
    ctx: &egui::Context,
    tokens: &GlassTokens,
    manga: &Manga,
    progress: &ReadingProgress,
) -> bool {
    let mut clicked = false;
    egui::Area::new(egui::Id::new("floating_continue_reading_pill"))
        .anchor(Align2::RIGHT_BOTTOM, Vec2::new(-24.0, -24.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(tokens.primary)
                .rounding(Rounding::same(20.0))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 60)))
                .inner_margin(egui::Margin::symmetric(18.0, 11.0))
                .show(ui, |ui| {
                    let resp = ui.interact(ui.max_rect(), egui::Id::new("cont_pill_hit"), Sense::click());
                    if resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("▶").size(18.0).color(tokens.on_primary).strong());
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            let short_title = truncate_str(&manga.title, 26);
                            if ui
                                .add(
                                    egui::Label::new(
                                        RichText::new(format!("Continue: {}", short_title))
                                            .size(13.0)
                                            .strong()
                                            .color(tokens.on_primary),
                                    )
                                    .sense(Sense::click()),
                                )
                                .clicked()
                            {
                                clicked = true;
                            }
                            let ch_str = match progress.chapter_number {
                                Some(n) if (n.fract()).abs() < 0.001 => format!("Chapter {}", n as i32),
                                Some(n) => format!("Chapter {:.1}", n),
                                None => "Current Chapter".to_string(),
                            };
                            let sub = format!("{} • Page {}", ch_str, progress.page_index + 1);
                            if ui
                                .add(
                                    egui::Label::new(
                                        RichText::new(sub)
                                            .size(11.0)
                                            .color(tokens.on_primary.gamma_multiply(0.85)),
                                    )
                                    .sense(Sense::click()),
                                )
                                .clicked()
                            {
                                clicked = true;
                            }
                        });
                    });
                    if resp.clicked() {
                        clicked = true;
                    }
                });
        });
    clicked
}

pub fn truncate_str(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}
