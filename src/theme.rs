use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeId {
    #[default]
    BtopNeon,        // Transparent 1: Vivid Cyan & Violet (Default)
    CatppuccinMocha, // Transparent 2: Pastel Lavender, Mauve & Peach (Catppuccin)
    TokyoNight,      // Transparent 3: Sleek Storm Navy, Cyan & Violet
    Dracula,         // Transparent 4: Iconic Vampire Purple, Pink & Green
    GruvboxDark,     // Transparent 5: Earthy Retro Warm Aqua & Gold
    RosePine,        // Transparent 6: Aesthetic Muted Pine, Rose & Gold
    CyberMatrix,     // Transparent 7: Emerald Green & Laser Amber
    Synthwave,       // Transparent 8: Hot Magenta & Electric Cyan
    NordFrost,       // Transparent 9: Glacier Ice & Aurora Gold
    SolarizedGlow,   // Transparent 10: Solarized Teal & Amber Gold
    MidnightOled,    // Solid Fill 1: Deep Opaque Midnight Black
    LightPaper,      // Solid Fill 2: Pure Opaque Light Paper
}

impl ThemeId {
    pub fn all() -> &'static [ThemeId] {
        &[
            ThemeId::BtopNeon,
            ThemeId::CatppuccinMocha,
            ThemeId::TokyoNight,
            ThemeId::Dracula,
            ThemeId::GruvboxDark,
            ThemeId::RosePine,
            ThemeId::CyberMatrix,
            ThemeId::Synthwave,
            ThemeId::NordFrost,
            ThemeId::SolarizedGlow,
            ThemeId::MidnightOled,
            ThemeId::LightPaper,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::BtopNeon => "Btop Neon [Transparent]",
            Self::CatppuccinMocha => "Catppuccin Mocha [Transparent]",
            Self::TokyoNight => "Tokyo Night [Transparent]",
            Self::Dracula => "Dracula [Transparent]",
            Self::GruvboxDark => "Gruvbox Dark [Transparent]",
            Self::RosePine => "Rosé Pine [Transparent]",
            Self::CyberMatrix => "Cyber Matrix [Transparent]",
            Self::Synthwave => "Synthwave 80s [Transparent]",
            Self::NordFrost => "Nord Frost [Transparent]",
            Self::SolarizedGlow => "Solarized Glow [Transparent]",
            Self::MidnightOled => "Midnight OLED [Solid Fill 1]",
            Self::LightPaper => "Light Paper [Solid Fill 2]",
        }
    }

    pub fn is_solid(&self) -> bool {
        matches!(self, Self::MidnightOled | Self::LightPaper)
    }

    pub fn next(&self) -> Self {
        match self {
            Self::BtopNeon => Self::CatppuccinMocha,
            Self::CatppuccinMocha => Self::TokyoNight,
            Self::TokyoNight => Self::Dracula,
            Self::Dracula => Self::GruvboxDark,
            Self::GruvboxDark => Self::RosePine,
            Self::RosePine => Self::CyberMatrix,
            Self::CyberMatrix => Self::Synthwave,
            Self::Synthwave => Self::NordFrost,
            Self::NordFrost => Self::SolarizedGlow,
            Self::SolarizedGlow => Self::MidnightOled,
            Self::MidnightOled => Self::LightPaper,
            Self::LightPaper => Self::BtopNeon,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Self::BtopNeon => Self::LightPaper,
            Self::CatppuccinMocha => Self::BtopNeon,
            Self::TokyoNight => Self::CatppuccinMocha,
            Self::Dracula => Self::TokyoNight,
            Self::GruvboxDark => Self::Dracula,
            Self::RosePine => Self::GruvboxDark,
            Self::CyberMatrix => Self::RosePine,
            Self::Synthwave => Self::CyberMatrix,
            Self::NordFrost => Self::Synthwave,
            Self::SolarizedGlow => Self::NordFrost,
            Self::MidnightOled => Self::SolarizedGlow,
            Self::LightPaper => Self::MidnightOled,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackgroundMode {
    #[default]
    Transparent, // 1. Preserves host terminal transparency & blur
    ThemeTone,   // 2. Solid fill with subtle palette tone matching the theme
    PureBlack,   // 3. Deep pure pitch black (#000000 / RGB 0, 0, 0)
}

impl BackgroundMode {
    pub fn all() -> &'static [BackgroundMode] {
        &[
            BackgroundMode::Transparent,
            BackgroundMode::ThemeTone,
            BackgroundMode::PureBlack,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Transparent => "Transparent (Terminal Default)",
            Self::ThemeTone => "Theme Tone (Subtle Palette Tint)",
            Self::PureBlack => "Pure Black (Pitch / OLED #000000)",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Transparent => Self::ThemeTone,
            Self::ThemeTone => Self::PureBlack,
            Self::PureBlack => Self::Transparent,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Self::Transparent => Self::PureBlack,
            Self::ThemeTone => Self::Transparent,
            Self::PureBlack => Self::ThemeTone,
        }
    }
}

fn resolve_bg(id: ThemeId, bg_mode: BackgroundMode, theme_tone: Color) -> Option<Color> {
    match bg_mode {
        BackgroundMode::PureBlack => Some(Color::Rgb(0, 0, 0)),
        BackgroundMode::ThemeTone => Some(theme_tone),
        BackgroundMode::Transparent => {
            if id.is_solid() {
                Some(theme_tone)
            } else {
                None
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub id: ThemeId,
    pub name: &'static str,
    pub bg: Option<Color>,
    pub fg: Color,
    pub fg_dim: Color,
    pub fg_highlight: Color,
    pub border_normal: Color,
    pub border_active: Color,
    pub box_cpu: Color,
    pub box_gpu: Color,
    pub box_llm: Color,
    pub box_queue: Color,
    pub bar_track: Color,
    pub bar_fill: Color,
    pub spark_tps: Color,
    pub spark_compute: Color,
    pub spark_mem: Color,
    pub temp_cool: Color,
    pub temp_warm: Color,
    pub temp_hot: Color,
    pub status_online: Color,
    pub status_offline: Color,
    pub selected_bg: Color,
    pub selected_fg: Color,
}

impl Theme {
    pub fn get(id: ThemeId, bg_mode: BackgroundMode) -> Self {
        match id {
            // ================================================================
            // TRANSPARENT THEME 1: Btop Neon (Cyberpunk Violet & Cyan)
            // ================================================================
            ThemeId::BtopNeon => Self {
                id,
                name: id.name(),
                bg: resolve_bg(id, bg_mode, Color::Rgb(15, 17, 26)),
                fg: Color::Rgb(245, 248, 255),          // Crisp luminous white
                fg_dim: Color::Rgb(175, 190, 215),      // High-contrast readable silver
                fg_highlight: Color::Rgb(255, 225, 90),  // Bright gold
                border_normal: Color::Rgb(160, 110, 235), // Btop purple border
                border_active: Color::Rgb(70, 230, 250), // Bright neon cyan
                box_cpu: Color::Rgb(190, 120, 255),
                box_gpu: Color::Rgb(140, 110, 250),
                box_llm: Color::Rgb(70, 235, 200),
                box_queue: Color::Rgb(255, 165, 70),
                bar_track: Color::Rgb(75, 85, 110),
                bar_fill: Color::Rgb(80, 235, 150),
                spark_tps: Color::Rgb(255, 180, 60),
                spark_compute: Color::Rgb(255, 95, 170),
                spark_mem: Color::Rgb(70, 220, 250),
                temp_cool: Color::Rgb(90, 235, 150),
                temp_warm: Color::Rgb(255, 180, 60),
                temp_hot: Color::Rgb(255, 85, 85),
                status_online: Color::Rgb(80, 245, 130),
                status_offline: Color::Rgb(255, 80, 80),
                selected_bg: Color::Rgb(60, 50, 95),
                selected_fg: Color::Rgb(255, 255, 255),
            },

            // ================================================================
            // TRANSPARENT THEME 2: Catppuccin Mocha (Pastel Lavender, Mauve & Peach)
            // ================================================================
            ThemeId::CatppuccinMocha => Self {
                id,
                name: id.name(),
                bg: resolve_bg(id, bg_mode, Color::Rgb(30, 30, 46)),
                fg: Color::Rgb(205, 214, 244),          // Text #cdd6f4
                fg_dim: Color::Rgb(166, 173, 200),      // Subtext0 #a6adc8
                fg_highlight: Color::Rgb(249, 226, 175),// Yellow #f9e2af
                border_normal: Color::Rgb(88, 91, 112), // Surface2 #585b70
                border_active: Color::Rgb(180, 190, 254),// Lavender #b4befe
                box_cpu: Color::Rgb(203, 166, 247),    // Mauve #cba6f7
                box_gpu: Color::Rgb(137, 180, 250),    // Blue #89b4fa
                box_llm: Color::Rgb(148, 226, 213),    // Teal #94e2d5
                box_queue: Color::Rgb(250, 179, 135),   // Peach #fab387
                bar_track: Color::Rgb(49, 50, 68),      // Surface0 #313244
                bar_fill: Color::Rgb(166, 227, 161),    // Green #a6e3a1
                spark_tps: Color::Rgb(250, 179, 135),   // Peach
                spark_compute: Color::Rgb(203, 166, 247), // Mauve
                spark_mem: Color::Rgb(116, 199, 236),   // Sapphire #74c7ec
                temp_cool: Color::Rgb(166, 227, 161),   // Green
                temp_warm: Color::Rgb(249, 226, 175),   // Yellow
                temp_hot: Color::Rgb(243, 139, 168),    // Red #f38ba8
                status_online: Color::Rgb(166, 227, 161),
                status_offline: Color::Rgb(243, 139, 168),
                selected_bg: Color::Rgb(69, 71, 90),    // Surface1 #45475a
                selected_fg: Color::Rgb(205, 214, 244),
            },

            // ================================================================
            // TRANSPARENT THEME 3: Tokyo Night (Storm Navy, Cyan & Violet)
            // ================================================================
            ThemeId::TokyoNight => Self {
                id,
                name: id.name(),
                bg: resolve_bg(id, bg_mode, Color::Rgb(26, 27, 38)),
                fg: Color::Rgb(192, 202, 245),          // #c0caf5
                fg_dim: Color::Rgb(140, 153, 200),
                fg_highlight: Color::Rgb(224, 175, 104),// #e0af68
                border_normal: Color::Rgb(65, 72, 104), // #414868
                border_active: Color::Rgb(122, 162, 247),// #7aa2f7
                box_cpu: Color::Rgb(187, 154, 247),    // #bb9af7
                box_gpu: Color::Rgb(125, 207, 255),    // #7dcfff
                box_llm: Color::Rgb(115, 218, 202),    // #73daca
                box_queue: Color::Rgb(255, 158, 100),   // #ff9e64
                bar_track: Color::Rgb(41, 46, 66),
                bar_fill: Color::Rgb(115, 218, 202),
                spark_tps: Color::Rgb(255, 158, 100),
                spark_compute: Color::Rgb(187, 154, 247),
                spark_mem: Color::Rgb(125, 207, 255),
                temp_cool: Color::Rgb(115, 218, 202),
                temp_warm: Color::Rgb(224, 175, 104),
                temp_hot: Color::Rgb(247, 118, 142),    // #f7768e
                status_online: Color::Rgb(115, 218, 202),
                status_offline: Color::Rgb(247, 118, 142),
                selected_bg: Color::Rgb(47, 53, 80),
                selected_fg: Color::Rgb(192, 202, 245),
            },

            // ================================================================
            // TRANSPARENT THEME 4: Dracula (Iconic Vampire Purple, Pink & Green)
            // ================================================================
            ThemeId::Dracula => Self {
                id,
                name: id.name(),
                bg: resolve_bg(id, bg_mode, Color::Rgb(40, 42, 54)),
                fg: Color::Rgb(248, 248, 242),          // #f8f8f2
                fg_dim: Color::Rgb(189, 195, 210),
                fg_highlight: Color::Rgb(241, 250, 140),// #f1fa8c
                border_normal: Color::Rgb(98, 114, 164), // #6272a4
                border_active: Color::Rgb(189, 147, 249),// #bd93f9
                box_cpu: Color::Rgb(255, 121, 198),    // #ff79c6
                box_gpu: Color::Rgb(189, 147, 249),    // #bd93f9
                box_llm: Color::Rgb(139, 233, 253),    // #8be9fd
                box_queue: Color::Rgb(255, 184, 108),   // #ffb86c
                bar_track: Color::Rgb(68, 71, 90),      // #44475a
                bar_fill: Color::Rgb(80, 250, 123),     // #50fa7b
                spark_tps: Color::Rgb(255, 184, 108),
                spark_compute: Color::Rgb(255, 121, 198),
                spark_mem: Color::Rgb(139, 233, 253),
                temp_cool: Color::Rgb(80, 250, 123),
                temp_warm: Color::Rgb(241, 250, 140),
                temp_hot: Color::Rgb(255, 85, 85),      // #ff5555
                status_online: Color::Rgb(80, 250, 123),
                status_offline: Color::Rgb(255, 85, 85),
                selected_bg: Color::Rgb(68, 71, 90),
                selected_fg: Color::Rgb(248, 248, 242),
            },

            // ================================================================
            // TRANSPARENT THEME 5: Gruvbox Dark (Earthy Retro Warm Aqua & Gold)
            // ================================================================
            ThemeId::GruvboxDark => Self {
                id,
                name: id.name(),
                bg: resolve_bg(id, bg_mode, Color::Rgb(40, 40, 40)),
                fg: Color::Rgb(235, 219, 178),          // #ebdbb2
                fg_dim: Color::Rgb(168, 153, 132),      // #a89984
                fg_highlight: Color::Rgb(250, 189, 47), // #fabd2f
                border_normal: Color::Rgb(102, 92, 84), // #665c54
                border_active: Color::Rgb(254, 128, 25), // #fe8019
                box_cpu: Color::Rgb(211, 134, 155),    // #d3869b
                box_gpu: Color::Rgb(131, 165, 152),    // #83a598
                box_llm: Color::Rgb(142, 192, 124),    // #8ec07c
                box_queue: Color::Rgb(254, 128, 25),    // #fe8019
                bar_track: Color::Rgb(60, 56, 54),      // #3c3836
                bar_fill: Color::Rgb(184, 187, 38),     // #b8bb26
                spark_tps: Color::Rgb(254, 128, 25),
                spark_compute: Color::Rgb(211, 134, 155),
                spark_mem: Color::Rgb(131, 165, 152),
                temp_cool: Color::Rgb(184, 187, 38),
                temp_warm: Color::Rgb(250, 189, 47),
                temp_hot: Color::Rgb(251, 73, 52),      // #fb4934
                status_online: Color::Rgb(184, 187, 38),
                status_offline: Color::Rgb(251, 73, 52),
                selected_bg: Color::Rgb(80, 73, 69),
                selected_fg: Color::Rgb(235, 219, 178),
            },

            // ================================================================
            // TRANSPARENT THEME 6: Rosé Pine (Aesthetic Muted Pine, Rose & Gold)
            // ================================================================
            ThemeId::RosePine => Self {
                id,
                name: id.name(),
                bg: resolve_bg(id, bg_mode, Color::Rgb(25, 23, 36)),
                fg: Color::Rgb(224, 222, 244),          // #e0def4
                fg_dim: Color::Rgb(144, 140, 170),      // #908caa
                fg_highlight: Color::Rgb(246, 193, 119),// #f6c177
                border_normal: Color::Rgb(82, 79, 103), // #524f67
                border_active: Color::Rgb(156, 207, 216),// #9ccfd8
                box_cpu: Color::Rgb(196, 167, 231),    // #c4a7e7
                box_gpu: Color::Rgb(49, 116, 143),     // #31748f
                box_llm: Color::Rgb(156, 207, 216),    // #9ccfd8
                box_queue: Color::Rgb(235, 188, 186),   // #ebbcba
                bar_track: Color::Rgb(38, 35, 58),      // #26233a
                bar_fill: Color::Rgb(156, 207, 216),    // #9ccfd8
                spark_tps: Color::Rgb(246, 193, 119),
                spark_compute: Color::Rgb(196, 167, 231),
                spark_mem: Color::Rgb(156, 207, 216),
                temp_cool: Color::Rgb(156, 207, 216),
                temp_warm: Color::Rgb(246, 193, 119),
                temp_hot: Color::Rgb(235, 111, 146),    // #eb6f92
                status_online: Color::Rgb(156, 207, 216),
                status_offline: Color::Rgb(235, 111, 146),
                selected_bg: Color::Rgb(64, 61, 82),
                selected_fg: Color::Rgb(224, 222, 244),
            },

            // ================================================================
            // TRANSPARENT THEME 7: Cyber Matrix (Phosphor Green & Gold)
            // ================================================================
            ThemeId::CyberMatrix => Self {
                id,
                name: id.name(),
                bg: resolve_bg(id, bg_mode, Color::Rgb(10, 20, 15)),
                fg: Color::Rgb(240, 255, 245),          // Light phosphor green-white
                fg_dim: Color::Rgb(160, 215, 180),      // Mint luminous text
                fg_highlight: Color::Rgb(255, 230, 75),  // Cyber electric yellow
                border_normal: Color::Rgb(50, 180, 100), // Emerald green border
                border_active: Color::Rgb(70, 255, 140), // Laser lime active
                box_cpu: Color::Rgb(60, 235, 120),
                box_gpu: Color::Rgb(100, 215, 255),
                box_llm: Color::Rgb(255, 215, 60),
                box_queue: Color::Rgb(255, 140, 50),
                bar_track: Color::Rgb(45, 85, 65),
                bar_fill: Color::Rgb(70, 255, 140),
                spark_tps: Color::Rgb(255, 225, 70),
                spark_compute: Color::Rgb(70, 255, 140),
                spark_mem: Color::Rgb(90, 220, 255),
                temp_cool: Color::Rgb(70, 255, 140),
                temp_warm: Color::Rgb(255, 215, 60),
                temp_hot: Color::Rgb(255, 80, 80),
                status_online: Color::Rgb(70, 255, 140),
                status_offline: Color::Rgb(255, 80, 80),
                selected_bg: Color::Rgb(30, 80, 50),
                selected_fg: Color::Rgb(240, 255, 245),
            },

            // ================================================================
            // TRANSPARENT THEME 8: Synthwave 80s (Hot Magenta & Neon Cyan)
            // ================================================================
            ThemeId::Synthwave => Self {
                id,
                name: id.name(),
                bg: resolve_bg(id, bg_mode, Color::Rgb(20, 12, 28)),
                fg: Color::Rgb(255, 245, 255),          // Radiant pearl white
                fg_dim: Color::Rgb(210, 180, 225),      // Luminous lavender
                fg_highlight: Color::Rgb(255, 235, 85),  // Neon sunny yellow
                border_normal: Color::Rgb(220, 60, 160), // Hot magenta border
                border_active: Color::Rgb(50, 230, 255), // Electric neon cyan
                box_cpu: Color::Rgb(255, 75, 175),
                box_gpu: Color::Rgb(175, 85, 255),
                box_llm: Color::Rgb(50, 230, 255),
                box_queue: Color::Rgb(255, 150, 50),
                bar_track: Color::Rgb(95, 60, 100),
                bar_fill: Color::Rgb(255, 75, 175),
                spark_tps: Color::Rgb(255, 150, 50),
                spark_compute: Color::Rgb(255, 75, 175),
                spark_mem: Color::Rgb(50, 230, 255),
                temp_cool: Color::Rgb(50, 230, 255),
                temp_warm: Color::Rgb(255, 160, 60),
                temp_hot: Color::Rgb(255, 60, 100),
                status_online: Color::Rgb(70, 240, 180),
                status_offline: Color::Rgb(255, 60, 100),
                selected_bg: Color::Rgb(95, 40, 95),
                selected_fg: Color::Rgb(255, 245, 255),
            },

            // ================================================================
            // TRANSPARENT THEME 9: Nord Frost (Arctic Ice & Aurora)
            // ================================================================
            ThemeId::NordFrost => Self {
                id,
                name: id.name(),
                bg: resolve_bg(id, bg_mode, Color::Rgb(46, 52, 64)),
                fg: Color::Rgb(240, 244, 250),          // Snow storm white
                fg_dim: Color::Rgb(180, 195, 215),      // Frost silver
                fg_highlight: Color::Rgb(235, 203, 139), // Aurora gold
                border_normal: Color::Rgb(110, 140, 180),// Nordic steel border
                border_active: Color::Rgb(136, 192, 208),// Glacier ice active
                box_cpu: Color::Rgb(129, 161, 193),
                box_gpu: Color::Rgb(180, 142, 173),
                box_llm: Color::Rgb(143, 188, 187),
                box_queue: Color::Rgb(235, 203, 139),
                bar_track: Color::Rgb(80, 95, 120),
                bar_fill: Color::Rgb(163, 190, 140),
                spark_tps: Color::Rgb(235, 203, 139),
                spark_compute: Color::Rgb(180, 142, 173),
                spark_mem: Color::Rgb(136, 192, 208),
                temp_cool: Color::Rgb(163, 190, 140),
                temp_warm: Color::Rgb(235, 203, 139),
                temp_hot: Color::Rgb(191, 97, 106),
                status_online: Color::Rgb(163, 190, 140),
                status_offline: Color::Rgb(191, 97, 106),
                selected_bg: Color::Rgb(67, 85, 110),
                selected_fg: Color::Rgb(240, 244, 250),
            },

            // ================================================================
            // TRANSPARENT THEME 10: Solarized Glow (Teal & Golden Amber)
            // ================================================================
            ThemeId::SolarizedGlow => Self {
                id,
                name: id.name(),
                bg: resolve_bg(id, bg_mode, Color::Rgb(0, 43, 54)),
                fg: Color::Rgb(253, 246, 227),          // Solarized base3 ivory
                fg_dim: Color::Rgb(180, 195, 195),      // Luminous base1
                fg_highlight: Color::Rgb(215, 165, 20),  // Solarized yellow
                border_normal: Color::Rgb(100, 140, 150),
                border_active: Color::Rgb(42, 185, 175), // Solarized cyan
                box_cpu: Color::Rgb(45, 160, 235),
                box_gpu: Color::Rgb(130, 135, 220),
                box_llm: Color::Rgb(42, 185, 175),
                box_queue: Color::Rgb(230, 100, 40),
                bar_track: Color::Rgb(70, 95, 100),
                bar_fill: Color::Rgb(150, 180, 20),
                spark_tps: Color::Rgb(215, 165, 20),
                spark_compute: Color::Rgb(230, 70, 150),
                spark_mem: Color::Rgb(42, 185, 175),
                temp_cool: Color::Rgb(150, 180, 20),
                temp_warm: Color::Rgb(215, 165, 20),
                temp_hot: Color::Rgb(220, 50, 47),
                status_online: Color::Rgb(150, 180, 20),
                status_offline: Color::Rgb(220, 50, 47),
                selected_bg: Color::Rgb(40, 80, 85),
                selected_fg: Color::Rgb(253, 246, 227),
            },

            // ================================================================
            // SOLID FILLED THEME 1: Midnight OLED (Deep Opaque Black Canvas)
            // ================================================================
            ThemeId::MidnightOled => Self {
                id,
                name: id.name(),
                bg: if bg_mode == BackgroundMode::PureBlack { Some(Color::Rgb(0, 0, 0)) } else { Some(Color::Rgb(12, 14, 20)) },
                fg: Color::Rgb(245, 248, 255),
                fg_dim: Color::Rgb(160, 175, 200),
                fg_highlight: Color::Rgb(255, 220, 85),
                border_normal: Color::Rgb(130, 90, 200),
                border_active: Color::Rgb(60, 220, 240),
                box_cpu: Color::Rgb(175, 105, 255),
                box_gpu: Color::Rgb(130, 95, 235),
                box_llm: Color::Rgb(70, 225, 190),
                box_queue: Color::Rgb(255, 155, 60),
                bar_track: Color::Rgb(40, 45, 60),
                bar_fill: Color::Rgb(75, 225, 140),
                spark_tps: Color::Rgb(255, 170, 50),
                spark_compute: Color::Rgb(255, 85, 160),
                spark_mem: Color::Rgb(60, 210, 240),
                temp_cool: Color::Rgb(90, 225, 140),
                temp_warm: Color::Rgb(255, 170, 50),
                temp_hot: Color::Rgb(255, 75, 75),
                status_online: Color::Rgb(75, 235, 120),
                status_offline: Color::Rgb(255, 70, 70),
                selected_bg: Color::Rgb(50, 45, 80),
                selected_fg: Color::Rgb(255, 255, 255),
            },

            // ================================================================
            // SOLID FILLED THEME 2: Light Paper (Daylight Off-White Canvas)
            // ================================================================
            ThemeId::LightPaper => Self {
                id,
                name: id.name(),
                bg: if bg_mode == BackgroundMode::PureBlack { Some(Color::Rgb(0, 0, 0)) } else { Some(Color::Rgb(246, 248, 252)) },
                fg: Color::Rgb(20, 24, 35),              // Dark ink
                fg_dim: Color::Rgb(95, 105, 125),        // Crisp readable graphite
                fg_highlight: Color::Rgb(180, 70, 0),    // Dark amber
                border_normal: Color::Rgb(170, 180, 200),
                border_active: Color::Rgb(30, 100, 220), // Ink blue
                box_cpu: Color::Rgb(40, 80, 190),
                box_gpu: Color::Rgb(120, 40, 180),
                box_llm: Color::Rgb(20, 130, 90),
                box_queue: Color::Rgb(190, 80, 20),
                bar_track: Color::Rgb(220, 225, 235),
                bar_fill: Color::Rgb(30, 140, 80),
                spark_tps: Color::Rgb(190, 80, 20),
                spark_compute: Color::Rgb(130, 40, 170),
                spark_mem: Color::Rgb(20, 110, 180),
                temp_cool: Color::Rgb(30, 140, 80),
                temp_warm: Color::Rgb(190, 110, 20),
                temp_hot: Color::Rgb(200, 30, 40),
                status_online: Color::Rgb(30, 140, 80),
                status_offline: Color::Rgb(200, 30, 40),
                selected_bg: Color::Rgb(215, 228, 250),
                selected_fg: Color::Rgb(15, 25, 60),
            },
        }
    }
}
