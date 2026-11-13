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
            painter.circle_stroke(center, r, stroke);
            painter.line_segment([center, center + Vec2::new(0.0, -r * 0.55)], stroke);
            painter.line_segment([center, center + Vec2::new(r * 0.45, 0.0)], stroke);
        }
        NativeIcon::Favorites => {
            let left_c = center + Vec2::new(-r * 0.35, -r * 0.25);
            let right_c = center + Vec2::new(r * 0.35, -r * 0.25);
            let sub_r = r * 0.42;
            painter.circle_stroke(left_c, sub_r, stroke);
            painter.circle_stroke(right_c, sub_r, stroke);
            let bottom = center + Vec2::new(0.0, r * 0.75);
            painter.line_segment([left_c + Vec2::new(-sub_r * 0.85, sub_r * 0.5), bottom], stroke);
            painter.line_segment([right_c + Vec2::new(sub_r * 0.85, sub_r * 0.5), bottom], stroke);
        }
        NativeIcon::Explore => {
            painter.circle_stroke(center, r, stroke);
            let tip_n = center + Vec2::new(0.0, -r * 0.65);
            let tip_s = center + Vec2::new(0.0, r * 0.65);
            let tip_e = center + Vec2::new(r * 0.28, 0.0);
            let tip_w = center + Vec2::new(-r * 0.28, 0.0);
            painter.line_segment([tip_w, tip_n], stroke);
            painter.line_segment([tip_n, tip_e], stroke);
            painter.line_segment([tip_e, tip_s], stroke);
            painter.line_segment([tip_s, tip_w], stroke);
        }
        NativeIcon::Feed => {
            painter.circle_filled(center + Vec2::new(0.0, r * 0.45), 2.2, color);
            let arc_stroke = Stroke::new(stroke_w * 0.9, color);
            painter.line_segment([center + Vec2::new(-r * 0.4, -r * 0.05), center + Vec2::new(r * 0.4, -r * 0.05)], arc_stroke);
            painter.line_segment([center + Vec2::new(-r * 0.75, -r * 0.45), center + Vec2::new(r * 0.75, -r * 0.45)], arc_stroke);
        }
        NativeIcon::Downloads => {
            let tray_y = center.y + r * 0.65;
            let tray_w = r * 0.75;
            painter.line_segment([Pos2::new(center.x - tray_w, tray_y), Pos2::new(center.x + tray_w, tray_y)], stroke);
            let arrow_tip = center + Vec2::new(0.0, r * 0.35);
            painter.line_segment([center + Vec2::new(0.0, -r * 0.65), arrow_tip], stroke);
            painter.line_segment([Pos2::new(center.x - r * 0.35, arrow_tip.y - r * 0.35), arrow_tip], stroke);
            painter.line_segment([Pos2::new(center.x + r * 0.35, arrow_tip.y - r * 0.35), arrow_tip], stroke);
        }
        NativeIcon::Settings => {
            painter.circle_stroke(center, r * 0.45, stroke);
            for i in 0..6 {
                let angle = (i as f32) * std::f32::consts::PI / 3.0;
                let dir = Vec2::new(angle.cos(), angle.sin());
                painter.line_segment([center + dir * (r * 0.45), center + dir * (r * 0.85)], stroke);
            }
        }
        NativeIcon::Fullscreen => {
            let d = r * 0.75;
            let arm = r * 0.38;
            let tl = center + Vec2::new(-d, -d);
            painter.line_segment([tl, tl + Vec2::new(arm, 0.0)], stroke);
            painter.line_segment([tl, tl + Vec2::new(0.0, arm)], stroke);
            let tr = center + Vec2::new(d, -d);
            painter.line_segment([tr, tr + Vec2::new(-arm, 0.0)], stroke);
            painter.line_segment([tr, tr + Vec2::new(0.0, arm)], stroke);
            let bl = center + Vec2::new(-d, d);
            painter.line_segment([bl, bl + Vec2::new(arm, 0.0)], stroke);
            painter.line_segment([bl, bl + Vec2::new(0.0, -arm)], stroke);
            let br = center + Vec2::new(d, d);
            painter.line_segment([br, br + Vec2::new(-arm, 0.0)], stroke);
            painter.line_segment([br, br + Vec2::new(0.0, -arm)], stroke);
        }
        NativeIcon::Amoled => {
            painter.circle_stroke(center, r * 0.72, stroke);
            painter.circle_filled(center + Vec2::new(r * 0.35, -r * 0.15), r * 0.58, Color32::from_rgb(18, 18, 22));
        }
        NativeIcon::Back => {
            let tip = center + Vec2::new(-r * 0.35, 0.0);
            painter.line_segment([center + Vec2::new(r * 0.35, -r * 0.55), tip], stroke);
            painter.line_segment([center + Vec2::new(r * 0.35, r * 0.55), tip], stroke);
        }
        NativeIcon::Close => {
            let d = r * 0.52;
            painter.line_segment([center + Vec2::new(-d, -d), center + Vec2::new(d, d)], stroke);
            painter.line_segment([center + Vec2::new(d, -d), center + Vec2::new(-d, d)], stroke);
        }
        NativeIcon::Search => {
            let r_lens = r * 0.48;
            let c_lens = center + Vec2::new(-r * 0.2, -r * 0.2);
            painter.circle_stroke(c_lens, r_lens, stroke);
            let handle_start = c_lens + Vec2::new(r_lens * 0.707, r_lens * 0.707);
            let handle_end = center + Vec2::new(r * 0.75, r * 0.75);
            painter.line_segment([handle_start, handle_end], stroke);
        }
        NativeIcon::ZoomIn => {
            let d = r * 0.55;
            painter.line_segment([center + Vec2::new(-d, 0.0), center + Vec2::new(d, 0.0)], stroke);
            painter.line_segment([center + Vec2::new(0.0, -d), center + Vec2::new(0.0, d)], stroke);
        }
        NativeIcon::ZoomOut => {
            let d = r * 0.55;
            painter.line_segment([center + Vec2::new(-d, 0.0), center + Vec2::new(d, 0.0)], stroke);
        }
        NativeIcon::Refresh => {
            painter.circle_stroke(center, r * 0.65, stroke);
            let tip = center + Vec2::new(r * 0.65, 0.0);
            painter.line_segment([tip + Vec2::new(-r * 0.3, -r * 0.3), tip], stroke);
            painter.line_segment([tip + Vec2::new(r * 0.3, -r * 0.3), tip], stroke);
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

    let bg = if selected {
        tokens.primary_container
    } else if hovered {
        tokens.surface_container_high
    } else {
        Color32::TRANSPARENT
    };

    let border = if selected {
        Stroke::new(1.5_f32, tokens.primary)
    } else if hovered {
        Stroke::new(1.0_f32, tokens.border_glass)
    } else {
        Stroke::NONE
    };

    let fg = if selected {
        tokens.primary
    } else if hovered {
        tokens.text_primary
    } else {
        tokens.text_muted
    };

    let painter = ui.painter_at(rect);
    if bg != Color32::TRANSPARENT {
        painter.rect_filled(rect, Rounding::same(8.0), bg);
    }
    if border != Stroke::NONE {
        painter.rect_stroke(rect, Rounding::same(8.0), border);
    }

    paint_native_icon(&painter, rect, icon, fg, 1.6_f32);

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
    show_source_badge: bool,
) -> CardAction {
    let mut action = CardAction::None;
    let card_w = 148.0;
    let cover_h = 204.0;
    let total_h = cover_h + 44.0;

    let (outer_rect, resp) = ui.allocate_exact_size(Vec2::new(card_w, total_h), Sense::click());
    let hovered = resp.hovered();

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

    // Draw elevation glow/shadow behind cover_rect
    let shadow_rect = cover_rect
        .translate(Vec2::new(0.0, if hovered { 4.0 } else { 2.0 }))
        .expand(if hovered { 2.0 } else { 0.5 });
    ui.painter().rect_filled(
        shadow_rect,
        Rounding::same(13.0),
        tokens.card_shadow_color(hovered),
    );

    // Render cover image inside cover_rect
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

    // Subtle bottom gradient scrim on cover
    let scrim_rect = Rect::from_min_max(
        Pos2::new(cover_rect.min.x, cover_rect.max.y - 38.0),
        cover_rect.max,
    );
    painter.rect_filled(
        scrim_rect,
        Rounding {
            nw: 0.0,
            ne: 0.0,
            sw: 12.0,
            se: 12.0,
        },
        Color32::from_rgba_unmultiplied(6, 8, 12, 165),
    );

    // Top-left Source Badge
    if show_source_badge {
        let src_label: String = manga.source_id.chars().take(10).collect();
        let pill_w = (src_label.len() as f32 * 6.2 + 12.0).clamp(38.0, 86.0);
        let pill_rect = Rect::from_min_size(
            Pos2::new(cover_rect.min.x + 6.0, cover_rect.min.y + 6.0),
            Vec2::new(pill_w, 18.0),
        );
        painter.rect_filled(
            pill_rect,
            Rounding::same(6.0),
            Color32::from_rgba_unmultiplied(10, 12, 18, 210),
        );
        painter.rect_stroke(pill_rect, Rounding::same(6.0), Stroke::new(1.0_f32, tokens.border_glass));
        painter.text(
            pill_rect.center(),
            Align2::CENTER_CENTER,
            src_label,
            FontId::proportional(9.5),
            tokens.primary,
        );
    }

    // Top-right Manhwa badge if Manhwa
    if manga.is_manhwa {
        let badge_rect = Rect::from_min_size(
            Pos2::new(cover_rect.max.x - 52.0, cover_rect.min.y + 6.0),
            Vec2::new(46.0, 18.0),
        );
        painter.rect_filled(
            badge_rect,
            Rounding::same(6.0),
            Color32::from_rgba_unmultiplied(18, 24, 34, 215),
        );
        painter.text(
            badge_rect.center(),
            Align2::CENTER_CENTER,
            "WEBTOON",
            FontId::proportional(8.5),
            Color32::from_rgb(111, 221, 226),
        );
    }

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

    // Cover Border (glows in primary color on hover)
    let border_stroke = if hovered {
        Stroke::new(2.0_f32, tokens.primary)
    } else {
        Stroke::new(1.0_f32, tokens.border_glass)
    };
    painter.rect_stroke(cover_rect, Rounding::same(12.0), border_stroke);

    // 2-line Title below cover
    let title_color = if hovered {
        tokens.primary
    } else {
        tokens.text_primary
    };
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
