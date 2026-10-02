use nannou::color::Srgb;

#[derive(Debug, Clone, Copy)]
pub enum Color {
    Red,
    Orange,
    Yellow,
    Green,
    DarkGreen,
    Blue,
    DarkBlue,
    Purple,
    Pink,
    White,
    Grey,
}

impl Color {
    pub fn from_ident(ident: &str) -> Option<Color> {
        let color = match ident {
            "RED" => Color::Red,
            "ORANGE" => Color::Orange,
            "YELLOW" => Color::Yellow,
            "GREEN" => Color::Green,
            "DARKGREEN" => Color::DarkGreen,
            "BLUE" => Color::Blue,
            "DARKBLUE" => Color::DarkBlue,
            "PURPLE" => Color::Purple,
            "PINK" => Color::Pink,
            "WHITE" => Color::White,
            "GREY" => Color::Grey,
            _ => {
                return None;
            }
        };

        Some(color)
    }
}

impl From<Color> for Srgb<u8> {
    fn from(val: Color) -> Self {
        match val {
            Color::Red => nannou::color::RED,
            Color::Orange => nannou::color::ORANGE,
            Color::Yellow => nannou::color::YELLOW,
            Color::Green => nannou::color::GREEN,
            Color::DarkGreen => nannou::color::DARKGREEN,
            Color::Blue => nannou::color::BLUE,
            Color::DarkBlue => nannou::color::DARKBLUE,
            Color::Purple => nannou::color::PURPLE,
            Color::Pink => nannou::color::PINK,
            Color::White => nannou::color::WHITE,
            Color::Grey => nannou::color::GREY,
        }
    }
}
