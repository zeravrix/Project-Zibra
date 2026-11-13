use once_cell::sync::Lazy;
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use std::collections::{HashMap, HashSet};
use url::Url;
use zibra_core::MangaPage;

static TS_READER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?s)ts_reader\.run\s*\(\s*(\{.*?\})\s*\)"#).unwrap()
});
static CHAPTER_PRELOADED_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?s)(?:chapter_preloaded_images|chapter_images|image_list|pages_list|reader_images|chapImages)\s*=\s*(\[.*?\])"#).unwrap()
});
static EMBEDDED_IMG_URL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"https?://[^"'\s<>\\]+\.(?:webp|jpg|jpeg|png|avif)(?:\?[^"'\s<>\\]*)?"#).unwrap()
});
static CHAPTER_NUMBER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:chapter|ch|episode|ep|chapitre|capítulo|capitulo|الفصل|فصل)?\s*([0-9]+(?:\.[0-9]+)?)").unwrap()
});
static THUMB_SIZE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[-_]\d{2,5}x\d{2,5}(?=\.[a-zA-Z]{2,5}(?:\?|$))").unwrap()
});

pub fn parse_chapter_number(text: &str) -> Option<f32> {
    CHAPTER_NUMBER_REGEX
        .captures(text)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<f32>().ok())
}

pub fn normalize_cover_url(raw: &str) -> String {
    let cleaned_ws: String = raw.trim().chars().filter(|c| !c.is_control()).collect();
    let cleaned = THUMB_SIZE_REGEX.replace_all(&cleaned_ws, "");
    cleaned
        .replace("/thumb/", "/")
        .replace("/thumbs/", "/")
        .replace(" ", "%20")
}

pub fn extract_best_image_url(el: ElementRef<'_>, base_url: &str) -> Option<String> {
    let attrs = [
        "data-original",
        "data-full",
        "data-orig",
        "data-cfsrc",
        "data-src",
        "data-lazy-src",
        "bv-data-src",
        "data-url",
        "data-image",
        "data-echo",
        "src",
    ];

    for attr in attrs {
        if let Some(val) = el.value().attr(attr) {
            let trimmed: String = val.trim().chars().filter(|c| !c.is_control()).collect();
            let trimmed = trimmed.trim();
            if trimmed.is_empty()
                || trimmed.starts_with("data:")
                || trimmed.starts_with("javascript:")
                || trimmed.starts_with("about:")
            {
                continue;
            }
            if let Some(resolved) = resolve_url(base_url, trimmed) {
                if !is_junk_asset(&resolved) {
                    return Some(normalize_cover_url(&resolved));
                }
            }
        }
    }

    if let Some(srcset) = el.value().attr("srcset").or_else(|| el.value().attr("data-srcset")) {
        let best = srcset
            .split(',')
            .filter_map(|token| {
                let mut parts = token.trim().split_whitespace();
                let raw_url = parts.next()?;
                let score = parts
                    .next()
                    .and_then(|s| s.trim_end_matches('w').trim_end_matches('x').parse::<f32>().ok())
                    .unwrap_or(1.0);
                let resolved = resolve_url(base_url, raw_url)?;
                if is_junk_asset(&resolved) {
                    return None;
                }
                Some((resolved, score))
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(u, _)| normalize_cover_url(&u));
        if best.is_some() {
            return best;
        }
    }

    None
}

pub fn extract_chapter_pages(
    html: &str,
    chapter_url: &str,
    css_selectors: &str,
    extra_headers: &HashMap<String, String>,
) -> Vec<MangaPage> {
    let mut urls = Vec::new();

    // 1. Check MangaThemesia / MangaReader ts_reader.run({...}) payload
    if let Some(caps) = TS_READER_REGEX.captures(html) {
        if let Some(json_str) = caps.get(1) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str.as_str()) {
                if let Some(sources) = val.get("sources").and_then(|s| s.as_array()) {
                    if let Some(first_src) = sources.first() {
                        if let Some(imgs) = first_src.get("images").and_then(|i| i.as_array()) {
                            for img in imgs {
                                if let Some(u) = img.as_str() {
                                    if let Some(res) = resolve_url(chapter_url, u) {
                                        if !is_junk_asset(&res) {
                                            urls.push(res);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Check Madara / WP-Manga / custom JS preloaded arrays
    if urls.is_empty() {
        if let Some(caps) = CHAPTER_PRELOADED_REGEX.captures(html) {
            if let Some(json_str) = caps.get(1) {
                let cleaned = json_str.as_str().replace("\\/", "/");
                if let Ok(arr) = serde_json::from_str::<Vec<String>>(&cleaned) {
                    for u in arr {
                        if let Some(res) = resolve_url(chapter_url, &u) {
                            if !is_junk_asset(&res) {
                                urls.push(res);
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Parse DOM elements using source-specific CSS selectors
    if urls.is_empty() {
        let doc = Html::parse_document(html);
        if let Ok(sel) = Selector::parse(css_selectors) {
            for el in doc.select(&sel) {
                if let Some(u) = extract_best_image_url(el, chapter_url) {
                    urls.push(u);
                }
            }
        }
        // Fallback broad reader selectors if specific selector matched nothing
        if urls.is_empty() {
            let fallback_css = ".reading-content img, #readerarea img, .page-break img, .chapter-content img, .chapter-images img, .container-chapter-reader img, .vung-doc img, #arraydata img, div.image_list img, div.image_list canvas[data-src]";
            if let Ok(sel) = Selector::parse(fallback_css) {
                for el in doc.select(&sel) {
                    if let Some(u) = extract_best_image_url(el, chapter_url) {
                        urls.push(u);
                    }
                }
            }
        }
    }

    // 4. Last-resort script/HTML image URL scan for chapter CDN images
    if urls.is_empty() {
        let unescaped = html.replace("\\/", "/");
        for m in EMBEDDED_IMG_URL_REGEX.find_iter(&unescaped) {
            let candidate = m.as_str();
            if !is_junk_asset(candidate)
                && (candidate.contains("chapter")
                    || candidate.contains("uploads")
                    || candidate.contains("manga")
                    || candidate.contains("cdn")
                    || candidate.contains("data")
                    || candidate.contains("page"))
            {
                urls.push(candidate.to_string());
            }
        }
    }

    let mut seen = HashSet::new();
    urls.into_iter()
        .map(|u| u.trim().replace(' ', "%20"))
        .filter(|u| !u.is_empty() && seen.insert(u.clone()))
        .enumerate()
        .map(|(idx, url)| {
            MangaPage::new(idx, url, Some(chapter_url.to_string()))
                .with_headers(extra_headers.clone())
        })
        .collect()
}

pub fn resolve_url(base: &str, href: &str) -> Option<String> {
    let trimmed: String = href.trim().chars().filter(|c| !c.is_control()).collect();
    let trimmed = trimmed.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return Some(trimmed.to_string());
    }
    if trimmed.starts_with("//") {
        return Some(format!("https:{}", trimmed));
    }
    Url::parse(base)
        .ok()
        .and_then(|b| b.join(trimmed).ok())
        .map(|u| u.to_string())
}

pub fn is_junk_asset(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.contains("logo")
        || lower.contains("avatar")
        || lower.contains("favicon")
        || lower.contains("banner-ad")
        || lower.contains("discord")
        || lower.contains("gravatar")
        || lower.ends_with(".svg")
        || lower.ends_with(".ico")
        || lower.contains("loading.gif")
        || lower.contains("lazy_loading")
        || lower.contains("placeholder")
        || lower.contains("1x1")
}
