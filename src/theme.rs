use ratatui::style::{Color, Modifier, Style};

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub background: Color,
    pub surface: Color,
    pub panel: Color,
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub border: Color,
    pub muted: Color,
    pub focus: Color,
    pub break_color: Color,
}

impl Theme {
    pub const fn dark() -> Self {
        Self {
            background: Color::Rgb(12, 16, 20),
            surface: Color::Rgb(20, 26, 32),
            panel: Color::Rgb(25, 32, 39),
            primary: Color::Rgb(232, 238, 242),
            secondary: Color::Rgb(159, 174, 184),
            accent: Color::Rgb(101, 205, 168),
            success: Color::Rgb(126, 211, 151),
            warning: Color::Rgb(239, 190, 92),
            error: Color::Rgb(235, 118, 118),
            border: Color::Rgb(55, 70, 78),
            muted: Color::Rgb(101, 117, 127),
            focus: Color::Rgb(101, 205, 168),
            break_color: Color::Rgb(107, 184, 214),
        }
    }

    pub const fn light() -> Self {
        Self {
            background: Color::Rgb(245, 247, 248),
            surface: Color::Rgb(235, 240, 242),
            panel: Color::Rgb(226, 234, 237),
            primary: Color::Rgb(27, 40, 47),
            secondary: Color::Rgb(79, 98, 107),
            accent: Color::Rgb(18, 126, 93),
            success: Color::Rgb(32, 133, 73),
            warning: Color::Rgb(153, 101, 16),
            error: Color::Rgb(166, 50, 50),
            border: Color::Rgb(164, 181, 187),
            muted: Color::Rgb(112, 132, 141),
            focus: Color::Rgb(18, 126, 93),
            break_color: Color::Rgb(32, 111, 145),
        }
    }

    pub fn from_name(name: &str) -> Self {
        match name {
            "light" => Self::light(),
            _ => Self::dark(),
        }
    }

    pub fn title(self) -> Style {
        Style::default().fg(self.primary).add_modifier(Modifier::BOLD)
    }

    pub fn panel_style(self) -> Style {
        Style::default().bg(self.panel).fg(self.primary)
    }
}
