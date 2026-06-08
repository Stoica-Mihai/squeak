//! Color palettes. `Theme` is a flat struct of ratatui `Color`s; the app holds
//! one and cycles through `ALL` with `t`.

use ratatui::style::Color;

#[derive(Clone, Copy)]
pub struct Theme {
    pub name: &'static str,
    pub bg: Color,
    pub fg: Color,
    pub dim: Color,
    pub border: Color,
    pub accent: Color,
    pub sel_bg: Color,
    pub sel_fg: Color,
    pub ok: Color,
    pub err: Color,
}

const fn rgb(hex: u32) -> Color {
    Color::Rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

pub const MOCHA: Theme = Theme {
    name: "Mocha",
    bg: rgb(0x1e1e2e),
    fg: rgb(0xcdd6f4),
    dim: rgb(0x6c7086),
    border: rgb(0x45475a),
    accent: rgb(0x89b4fa),
    sel_bg: rgb(0x313244),
    sel_fg: rgb(0xf5e0dc),
    ok: rgb(0xa6e3a1),
    err: rgb(0xf38ba8),
};

pub const GRUVBOX: Theme = Theme {
    name: "Gruvbox",
    bg: rgb(0x282828),
    fg: rgb(0xebdbb2),
    dim: rgb(0x928374),
    border: rgb(0x504945),
    accent: rgb(0xfabd2f),
    sel_bg: rgb(0x3c3836),
    sel_fg: rgb(0xfbf1c7),
    ok: rgb(0xb8bb26),
    err: rgb(0xfb4934),
};

pub const NORD: Theme = Theme {
    name: "Nord",
    bg: rgb(0x2e3440),
    fg: rgb(0xd8dee9),
    dim: rgb(0x4c566a),
    border: rgb(0x434c5e),
    accent: rgb(0x88c0d0),
    sel_bg: rgb(0x3b4252),
    sel_fg: rgb(0xeceff4),
    ok: rgb(0xa3be8c),
    err: rgb(0xbf616a),
};

pub const TOKYO_NIGHT: Theme = Theme {
    name: "Tokyo Night",
    bg: rgb(0x1a1b26),
    fg: rgb(0xc0caf5),
    dim: rgb(0x565f89),
    border: rgb(0x414868),
    accent: rgb(0x7aa2f7),
    sel_bg: rgb(0x292e42),
    sel_fg: rgb(0xc0caf5),
    ok: rgb(0x9ece6a),
    err: rgb(0xf7768e),
};

pub const ALL: [Theme; 4] = [MOCHA, GRUVBOX, NORD, TOKYO_NIGHT];
