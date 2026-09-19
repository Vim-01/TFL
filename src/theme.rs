use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeId {
    #[default]
    BtopNeon,       // Transparent 1: Vivid Cyan & Violet
    CyberMatrix,    // Transparent 2: Emerald Green & Laser Amber
    Synthwave,      // Transparent 3: Hot Magenta & Electric Cyan
    NordFrost,      // Transparent 4: Glacier Ice & Aurora Gold
    SolarizedGlow,  // Transparent 5: Solarized Teal & Amber Gold
    MidnightOled,   // Solid Fill 1: Deep Opaque Midnight Black
    LightPaper,     // Solid Fill 2: Pure Opaque Light Paper
}

impl ThemeId {
    pub fn all() -> &'static [ThemeId] {
        &[
            ThemeId::BtopNeon,
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
            Self::BtopNeon => Self::CyberMatrix,
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
            Self::CyberMatrix => Self::BtopNeon,
            Self::Synthwave => Self::CyberMatrix,
            Self::NordFrost => Self::Synthwave,
            Self::SolarizedGlow => Self::NordFrost,
            Self::MidnightOled => Self::SolarizedGlow,
            Self::LightPaper => Self::MidnightOled,
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
    pub fn get(id: ThemeId, force_solid_bg: bool) -> Self {
        match id {
            // ================================================================
            // TRANSPARENT THEME 1: Btop Neon (Cyberpunk Violet & Cyan)
            // ================================================================
            ThemeId::BtopNeon => Self {
                id,
                name: id.name(),
                bg: if force_solid_bg { Some(Color::Rgb(15, 17, 26)) } else { None },
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
            // TRANSPARENT THEME 2: Cyber Matrix (Phosphor Green & Gold)
            // ================================================================
            ThemeId::CyberMatrix => Self {
                id,
                name: id.name(),
                bg: if force_solid_bg { Some(Color::Rgb(10, 20, 15)) } else { None },
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
            // TRANSPARENT THEME 3: Synthwave 80s (Hot Magenta & Neon Cyan)
            // ================================================================
            ThemeId::Synthwave => Self {
                id,
                name: id.name(),
                bg: if force_solid_bg { Some(Color::Rgb(20, 12, 28)) } else { None },
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
            // TRANSPARENT THEME 4: Nord Frost (Arctic Ice & Aurora)
            // ================================================================
            ThemeId::NordFrost => Self {
                id,
                name: id.name(),
                bg: if force_solid_bg { Some(Color::Rgb(46, 52, 64)) } else { None },
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
            // TRANSPARENT THEME 5: Solarized Glow (Teal & Golden Amber)
            // ================================================================
            ThemeId::SolarizedGlow => Self {
                id,
                name: id.name(),
                bg: if force_solid_bg { Some(Color::Rgb(0, 43, 54)) } else { None },
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
                bg: Some(Color::Rgb(12, 14, 20)),       // Always solid opaque black
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
                bg: Some(Color::Rgb(246, 248, 252)),     // Always solid opaque ivory
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
