//! SVG Icons Module
//!
//! Provides Heroicons-style SVG icon components for GPUI.
//! Based on Heroicons (https://heroicons.com/) MIT License.

use gpui::prelude::*;
use gpui::*;

/// Icon size variants
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IconSize {
    Xs,  // 12px (w-3 h-3)
    Sm,  // 16px (w-4 h-4)
    Md,  // 20px (w-5 h-5)
    Lg,  // 24px (w-6 h-6)
}

impl IconSize {
    pub fn size(&self) -> Pixels {
        match self {
            IconSize::Xs => px(12.0),
            IconSize::Sm => px(16.0),
            IconSize::Md => px(20.0),
            IconSize::Lg => px(24.0),
        }
    }
}

/// Stroke width variants
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StrokeWidth {
    Thin,   // 1.5
    Normal, // 2
    Thick,  // 2.5
}

impl StrokeWidth {
    pub fn width(&self) -> f32 {
        match self {
            StrokeWidth::Thin => 1.5,
            StrokeWidth::Normal => 2.0,
            StrokeWidth::Thick => 2.5,
        }
    }
}

/// SVG Icon component
pub struct SvgIcon {
    paths: Vec<IconPath>,
    size: IconSize,
    stroke_width: StrokeWidth,
    color: Hsla,
}

/// Icon path definition
#[derive(Debug, Clone)]
pub struct IconPath {
    /// SVG path data (d attribute)
    d: String,
    /// Stroke linecap (round/square/butt)
    linecap: String,
    /// Stroke linejoin (round/miter/bevel)
    linejoin: String,
}

impl IconPath {
    pub fn new(d: impl Into<String>) -> Self {
        Self {
            d: d.into(),
            linecap: "round".to_string(),
            linejoin: "round".to_string(),
        }
    }

    pub fn with_linecap(mut self, linecap: impl Into<String>) -> Self {
        self.linecap = linecap.into();
        self
    }

    pub fn with_linejoin(mut self, linejoin: impl Into<String>) -> Self {
        self.linejoin = linejoin.into();
        self
    }
}

impl SvgIcon {
    /// Create a new SVG icon with paths
    pub fn new(paths: Vec<IconPath>, size: IconSize, stroke_width: StrokeWidth, color: Hsla) -> Self {
        Self {
            paths,
            size,
            stroke_width,
            color,
        }
    }

    /// Create from a single path string
    pub fn from_path(d: impl Into<String>, size: IconSize, stroke_width: StrokeWidth, color: Hsla) -> Self {
        Self::new(vec![IconPath::new(d)], size, stroke_width, color)
    }
}

impl IntoElement for SvgIcon {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let size = self.size.size();
        let stroke_color = self.color;

        // GPUI doesn't have native SVG support yet, so we use a canvas-like approach
        // with positioned rectangles to approximate the icon shape
        // For a proper implementation, we would need to use gpui's canvas or custom rendering

        div()
            .size(size)
            .flex()
            .items_center()
            .justify_center()
            .child(
                // Placeholder: a colored rectangle approximating the icon
                // In production GPUI apps, use Canvas or custom SVG rendering
                div()
                    .size(px(self.stroke_width.width()))
                    .bg(stroke_color)
                    .rounded(px(self.stroke_width.width() / 2.0))
            )
    }
}

/// Pre-defined Heroicons (Outline style)
/// These are common icons used throughout the application

/// Arrow left icon (back button)
pub fn arrow_left_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M10.5 19.5 3 12m0 0 7.5-7.5M3 12h18",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Arrow right icon
pub fn arrow_right_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M13.5 4.5 21 12m0 0-7.5 7.5M21 12H3",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Chevron left icon (scroll left)
pub fn chevron_left_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M15.75 19.5 8.25 12l7.5-7.5",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Chevron right icon (scroll right)
pub fn chevron_right_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "m8.25 4.5 7.5 7.5-7.5 7.5",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Chevron up icon
pub fn chevron_up_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M4.5 15.75l7.5-7.5 7.5 7.5",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Chevron down icon
pub fn chevron_down_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M19.5 8.25l-7.5 7.5-7.5-7.5",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Plus icon (add/create)
pub fn plus_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M12 4.5v15m7.5-7.5h-15",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// X mark icon (close/cancel)
pub fn x_mark_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M6 18 18 6M6 6l12 12",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Magnifying glass icon (search)
pub fn magnifying_glass_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Clock icon (time)
pub fn clock_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M12 6v6h4.5m4.5 0a9 9 0 11-18 0 9 9 0 0118 0Z",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Paper airplane icon (send message)
pub fn paper_airplane_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M6 12L3.269 3.126A59.768 59.768 0 0121.485 12 59.77 59.77 0 013.27 20.876L5.999 12zm0 0h7.5",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Ellipsis vertical icon (more menu)
pub fn ellipsis_vertical_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M12 6.75a.75.75 0 110-1.5.75.75 0 010 1.5zM12 12.75a.75.75 0 110-1.5.75.75 0 010 1.5zM12 18.75a.75.75 0 110-1.5.75.75 0 010 1.5z",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Clipboard icon (copy)
pub fn clipboard_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M15.666 3.888A2.25 2.25 0 0 0 13.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 0 1-.75.75H9a.75.75 0 0 1-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 0 1 1.927-.184",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Arrow down tray icon (export/download)
pub fn arrow_down_tray_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5M16.5 12 12 16.5m0 0L7.5 12m4.5 4.5V3",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Trash icon (delete)
pub fn trash_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Server stack icon (clusters)
pub fn server_stack_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Refresh icon (reload)
pub fn refresh_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Exclamation circle icon (error/warning)
pub fn exclamation_circle_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Check circle icon (success)
pub fn check_circle_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M9 12.75 11.25 15 15 9.75M21 12a9 9 0 11-18 0 9 9 0 0118 0z",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Information circle icon (info)
pub fn information_circle_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "m11.25 11.25.041-.02a.75.75 0 011.063.852l-.708 2.836a.75.75 0 001.063.853l.041-.021M21 12a9 9 0 11-18 0 9 9 0 0118 0zm-9-3.75h.008v.008H12V8.25z",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// User group icon (consumer groups)
pub fn user_group_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M18 18.75a.75.75 0 0 0 .75-.75c0-.178-.012-.355-.036-.528A9.75 9.75 0 0 0 12 3.75c-1.324 0-2.595.274-3.75.772V18h9.75ZM12 2.25c-2.485 0-4.856.488-7.062 1.38a.75.75 0 0 0-.447.932l.958 3.758a.75.75 0 0 0 .973.536 8.25 8.25 0 0 1 10.572 0 .75.75 0 0 0 .973-.536l.958-3.758a.75.75 0 0 0-.447-.932A18.25 18.25 0 0 0 12 2.25Z",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Cylinder icon (database/kafka)
pub fn cylinder_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694-4.125-8.25-4.125s-8.25-1.847-8.25-4.125",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Star icon (favorite)
pub fn star_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M11.48 3.499a.562.562 0 011.04 0l2.125 5.111a.563.563 0 00.475.345l5.518.442c.499.04.701.663.321.988l-4.303 3.574a.563.563 0 00-.173.515l.672 5.663a.562.562 0 01-.808.559l-5.145-2.873a.563.563 0 00-.586 0L6.98 20.54a.562.562 0 01-.808-.56l.672-5.663a.563.563 0 00-.173-.515L2.34 8.789a.562.562 0 01.321-.988l5.518-.442a.563.563 0 00.475-.345L11.48 3.5z",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Gear/Settings icon
pub fn gear_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.325.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 011.37.49l1.296 2.247a1.125 1.125 0 01-.26 1.46l-.942.764c-.345.284-.55.687-.55 1.12 0 .24.054.47.14.682.085.21.2.407.34.583.278.357.418.798.418 1.265v1.5a1.125 1.125 0 01-1.125 1.125h-1.5a1.125 1.125 0 01-1.125-1.125v-1.5c0-.467-.14-.908-.418-1.265a2.36 2.36 0 00-.34-.583 2.36 2.36 0 00-.14-.682c0-.433.205-.836.55-1.12l.942-.764a1.125 1.125 0 00.26-1.46l-1.296-2.247a1.125 1.125 0 00-1.37-.49l-1.217.456c-.355.133-.75.072-1.075-.124a2.36 2.36 0 00-.22-.127c-.332-.184-.582-.496-.645-.87l-.213-1.281ZM12 18a3 3 0 100-6 3 3 0 000 6Z",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Globe/Language icon
pub fn globe_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M12 21a9.004 9.004 0 008.716-6.747M12 21a9.004 9.004 0 01-8.716-6.747M12 21c2.485 0 4.5-4.03 4.5-9S14.485 3 12 3m0 18c-2.485 0-4.5-4.03-4.5-9S9.515 3 12 3m0 0a8.997 8.997 0 017.843 4.582M12 3a8.997 8.997 0 00-7.843 4.582m15.686 0A11.953 11.953 0 0112 10.5c-2.998 0-5.74-1.1-7.843-2.918m15.686 0A8.959 8.959 0 0121 12c0 .778-.099 1.533-.284 2.253m0 0A17.935 17.935 0 0112 16.5c-3.206 0-6.09-.872-8.716-2.253m0 0A9.023 9.023 0 013 12c0-1.622.381-3.178 1.064-4.582",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Sun icon (light theme)
pub fn sun_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M12 3v2.25m6.364.386-1.691-1.691c-.247-.247-.58-.386-.927-.386H8.254c-.347 0-.68.139-.927.386L5.636 5.636M12 3v2.25m0 16.5v2.25m0-2.25c-4.97 0-9-4.03-9-9s4.03-9 9-9 9 4.03 9 9-4.03 9-9 9Zm6.364-4.386 1.691 1.691c.247.247.386.58.386.927v3.414c0 .347-.139.68-.386.927l-1.691 1.691M5.636 18.364l-1.691-1.691c-.247-.247-.386-.58-.386-.927V13.332c0-.347.139-.68.386-.927l1.691-1.691",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Moon icon (dark theme)
pub fn moon_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M21.752 15.002A9.718 9.718 0 0118 15.75c-5.385 0-9.75-4.365-9.75-9.75 0-1.33.266-2.597.748-3.752A9.753 9.753 0 003 11.25C3 16.635 7.365 21 12.75 21a9.753 9.753 0 006.752-4.998z",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Play icon (start)
pub fn play_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M5.25 5.653c0-.856.917-1.398 1.667-.986l11.54 6.348a1.125 1.125 0 010 1.986l-11.54 6.347a1.125 1.125 0 01-1.667-.985V5.653z",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Stop icon (stop)
pub fn stop_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M5.25 7.5A2.25 2.25 0 017.5 5.25h9a2.25 2.25 0 012.25 2.25v9a2.25 2.25 0 01-2.25 2.25h-9a2.25 2.25 0 01-2.25-2.25v-9z",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Sort arrows icon (up/down)
pub fn arrows_up_down_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M8.25 15L12 18.75 15.75 15m-7.5-6L12 5.25 15.75 9",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Eye icon (view)
pub fn eye_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M2.036 17.25a3.375 3.375 0 016.372-1.818M2.036 17.25a3.375 3.375 0 006.372 1.818M2.036 17.25a3.375 3.375 0 00-3.375-3.375H4.5M21 12a9 9 0 0118 0 9 9 0 01-18 0m0 0c0 5.374 3.95 9.75 9 9.75s9-4.376 9-9.75-3.95-9.75-9-9.75S0 6.626 0 12z",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Pencil icon (edit)
pub fn pencil_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "m16.862 4.487 1.687-1.688a1.875 1.875 0 112.652 2.652L6.832 19.82a4.5 4.5 0 01-1.897 1.13L2.685 21.5a.75.75 0 01-.447-.139.75.75 0 01-.139-.447l.55-2.25a4.5 4.5 0 011.13-1.897L16.863 4.487zm0 0L19.5 7.125",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Code bracket icon (json/code)
pub fn code_bracket_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M17.25 6.75L22.5 12l-5.25 5.25m-10.5 0L1.5 12l5.25-5.25m7.5-3l-4.5 16.5",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Home icon
pub fn home_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "m2.25 12 8.25-8.25a1.125 1.125 0 011.5 0l8.25 8.25M3.75 12v7.5a1.5 1.5 0 001.5 1.5h12a1.5 1.5 0 001.5-1.5V12",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Bookmark icon (history/saved)
pub fn bookmark_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M17.594 3.322c1.1.128 1.907 1.077 1.907 2.185V21L12 17.25 4.5 21V5.507c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 011.927-.184",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Folder icon
pub fn folder_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M2.25 12.75V12A2.25 2.25 0 014.5 9.75h15A2.25 2.25 0 0121.75 12v.75m-8.69-6.44-2.12-2.12a1.5 1.5 0 00-1.061-.44H4.5A2.25 2.25 0 002.25 6v12a2.25 2.25 0 002.25 2.25h15A2.25 2.25 0 0021.75 18V9a2.25 2.25 0 00-2.25-2.25h-5.379a1.5 1.5 0 00-1.06-.44z",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Document text icon
pub fn document_text_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9z",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Bars 3 icon (hamburger menu)
pub fn bars_3_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M3.75 6.75h16.5M3.75 12h16.5m-16.5 5.25h16.5",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Share icon
pub fn share_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M7.217 10.907a2.25 2.25 0 100 2.186m0-2.186c.18.324.283.696.283 1.093s-.103.77-.283 1.093m0-2.186 9.66-5.05m-9.66 7.981 9.66 5.05m-9.66-2.796 9.66-5.05m0-5.05 2.25 1.178m-2.25-1.178L21 12m-2.25-5.05 2.25 1.178M3.75 12m9.66-5.05 2.25 1.178m0 0 2.25 1.178m0 0v5.05m0-5.05-2.25 1.178",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Question mark circle icon (help)
pub fn question_mark_circle_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M9.879 7.519c1.171-1.025 3.071-1.025 4.242 0 1.172 1.025 1.172 2.687 0 3.712-.203.179-.43.326-.67.442-.745.361-1.45.999-1.45 1.827v.75M21 12a9 9 0 11-18 0 9 9 0 0118 0zm-9 5.25h.008v.008H12v-.008z",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Light bulb icon (tour/guide)
pub fn light_bulb_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M12 18v-5.25m0 0a6.01 6.01 0 001.5-.189m-1.5.189a6.01 6.01 0 01-1.5-.189m1.5.189h3.75m-3.75 0H8.5m3.75 0a6 6 0 01-3.75 0M12 18v.008h.008V18H12zm0 0h.008V18H12v-.008zm0 0v.008h.008V18H12z",
        size,
        StrokeWidth::Normal,
        color,
    )
}

/// Sparkles icon (tour highlight)
pub fn sparkles_icon(size: IconSize, color: Hsla) -> SvgIcon {
    SvgIcon::from_path(
        "M9.813 15.904 9 18.75l-.813-2.846a4.5 4.5 0 00-3.09-3.09L2.25 12l2.846-.813a4.5 4.5 0 003.09-3.09L9 5.25l.813 2.846a4.5 4.5 0 003.09 3.09L12 12l-2.846.813a4.5 4.5 0 00-3.09 3.09zM18.259 8.715 18 9.75l-.259-1.035a3.375 3.375 0 00-2.455-2.456L14.25 6l1.036-.259a3.375 3.375 0 002.455-2.456L18 2.25l.259 1.035a3.375 3.375 0 002.455 2.456L21.75 6l-1.036.259a3.375 3.375 0 00-2.455 2.456zM16.894 20.567 16.5 21.75l-.394-1.183a2.25 2.25 0 00-1.637-1.637L13.5 18.75l1.183-.394a2.25 2.25 0 001.637-1.637l.394-1.183.394 1.183a2.25 2.25 0 001.637 1.637l1.183.394-1.183.394a2.25 2.25 0 00-1.637 1.637z",
        size,
        StrokeWidth::Normal,
        color,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icon_size() {
        assert_eq!(IconSize::Xs.size(), px(12.0));
        assert_eq!(IconSize::Sm.size(), px(16.0));
        assert_eq!(IconSize::Md.size(), px(20.0));
        assert_eq!(IconSize::Lg.size(), px(24.0));
    }

    #[test]
    fn test_stroke_width() {
        assert_eq!(StrokeWidth::Thin.width(), 1.5);
        assert_eq!(StrokeWidth::Normal.width(), 2.0);
        assert_eq!(StrokeWidth::Thick.width(), 2.5);
    }

    #[test]
    fn test_icon_path_creation() {
        let path = IconPath::new("M12 4.5v15m7.5-7.5h-15");
        assert_eq!(path.d, "M12 4.5v15m7.5-7.5h-15");
        assert_eq!(path.linecap, "round");
        assert_eq!(path.linejoin, "round");
    }
}