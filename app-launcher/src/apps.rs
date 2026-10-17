//! The catalogue the grid shows.
//!
//! The launcher decides the identity, order, localized names, icon (glyph, bitmap, or QOI),
//! and accent color of all apps displayed on the home screen.

use pomelo_material_symbols::Icon;
pub use pomelo_widgets::{AppIcon, Language};

/// Strong-typed unique identifier for each application hosted by the launcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppId {
    Settings,
    WirelessClipboard,
    DailySchedule,
    Calculator,
    Terminal,
    Music,
    DemoCounter,
    DemoHello,
}

impl AppId {
    /// All 8 applications supported by the launcher.
    pub const ALL: [Self; 8] = [
        Self::Settings,
        Self::WirelessClipboard,
        Self::DailySchedule,
        Self::Calculator,
        Self::Terminal,
        Self::Music,
        Self::DemoCounter,
        Self::DemoHello,
    ];
}

/// Metadata describing an application entry in the launcher grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    pub id: AppId,
    pub name: &'static str,
    pub name_zh: &'static str,
    pub icon: AppIcon,
    pub accent: (u8, u8, u8),
}

impl Entry {
    /// Constructs a new [`Entry`].
    pub const fn new(
        id: AppId,
        name: &'static str,
        name_zh: &'static str,
        icon: AppIcon,
        accent: (u8, u8, u8),
    ) -> Self {
        Self {
            id,
            name,
            name_zh,
            icon,
            accent,
        }
    }

    /// Returns the localized name in `language`.
    pub const fn localized_name(&self, language: Language) -> &'static str {
        match language {
            Language::Chinese => self.name_zh,
            Language::English => self.name,
        }
    }

    /// The accent color, as an iced [`iced::Color`].
    pub fn color(&self) -> iced::Color {
        let (r, g, b) = self.accent;
        iced::Color::from_rgb8(r, g, b)
    }
}

/// Helper function to find an entry in [`CATALOGUE`] by its [`AppId`].
pub fn entry_of(id: AppId) -> &'static Entry {
    CATALOGUE
        .iter()
        .find(|e| e.id == id)
        .expect("Every AppId must exist in CATALOGUE")
}

/// Static QOI compressed icon for Hello app (118x118 RGBA with antialiased squircle).
const HELLO_QOI: &[u8] = include_bytes!("../../../assets/app-icons/hello.qoi");

/// Default wallpaper: Golden Night Milky Way (480x430 RGB QOI, Unsplash CC0).
pub const WALLPAPER_QOI: &[u8] = include_bytes!("../../../assets/image/wallpaper_milkyway.qoi");

/// Eight apps, in the order the grid shows them.
pub const CATALOGUE: &[Entry] = &[
    // Page 1
    // Row 1: Settings, Clipboard
    Entry {
        id: AppId::Settings,
        name: "Settings",
        name_zh: "设置",
        icon: AppIcon::glyph(Icon::SETTINGS),
        accent: (40, 52, 60),
    },
    Entry {
        id: AppId::WirelessClipboard,
        name: "Clipboard",
        name_zh: "无线剪切板",
        icon: AppIcon::glyph(Icon::CONTENT_PASTE),
        accent: (42, 60, 78),
    },
    // Row 2: Daily Schedule, Calculator
    Entry {
        id: AppId::DailySchedule,
        name: "Daily Schedule",
        name_zh: "日程",
        icon: AppIcon::glyph(Icon::SCHEDULE),
        accent: (44, 52, 68),
    },
    Entry {
        id: AppId::Calculator,
        name: "Calculator",
        name_zh: "计算器",
        icon: AppIcon::glyph(Icon::CALCULATE),
        accent: (46, 62, 46),
    },

    // Page 2
    // Row 1: Terminal, Music
    Entry {
        id: AppId::Terminal,
        name: "Terminal",
        name_zh: "终端",
        icon: AppIcon::glyph(Icon::TERMINAL),
        accent: (38, 44, 62),
    },
    Entry {
        id: AppId::Music,
        name: "Music",
        name_zh: "音乐",
        icon: AppIcon::glyph(Icon::MUSIC_NOTE),
        accent: (60, 40, 44),
    },
    // Row 2: Demo Counter, Demo Hello
    Entry {
        id: AppId::DemoCounter,
        name: "Demo Counter",
        name_zh: "演示 计数器",
        icon: AppIcon::glyph(Icon::COUNTER_0),
        accent: (62, 48, 36),
    },
    Entry {
        id: AppId::DemoHello,
        name: "Demo Hello",
        name_zh: "演示 你好",
        icon: AppIcon::qoi(HELLO_QOI),
        accent: (58, 38, 58),
    },
];
