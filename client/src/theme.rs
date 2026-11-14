//! Theme palettes. Generated from the colour blocks of the old stylesheet, one per theme id.

use iced::Color;

/// The colours of one theme.
#[derive(Clone, Copy, Debug)]
pub struct Pal {
    pub app_bg: Color,
    pub sidebar_bg: Color,
    pub rail_bg: Color,
    pub user_bar_bg: Color,
    pub input_bg: Color,
    pub hover_bg: Color,
    pub selected_bg: Color,
    pub text: Color,
    pub text_muted: Color,
    pub text_strong: Color,
    pub accent: Color,
    pub link: Color,
    pub green: Color,
    pub red: Color,
    pub on_color: Color,








    pub avatars: [Color; 8],
}

const fn c(hex: u32) -> Color {
    Color::from_rgb8((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

/// Looks up the palette for a theme id. Unknown ids get Breeze Dark.
pub fn palette(id: &str) -> Pal {
    match id {
        "breeze-dark" => Pal { app_bg: c(0x2a2e32), sidebar_bg: c(0x232629), rail_bg: c(0x1b1e20), user_bar_bg: c(0x1b1e20), input_bg: c(0x31363b), hover_bg: c(0x31363b), selected_bg: c(0x3d4348), text: c(0xd6d9dc), text_muted: c(0x7f8c8d), text_strong: c(0xfcfcfc), accent: c(0x3daee9), link: c(0x1d99f3), green: c(0x27ae60), red: c(0xda4453), on_color: c(0x1b1e20), avatars: [c(0x3daee9), c(0x9b59b6), c(0x27ae60), c(0xfdbc4b), c(0xda4453), c(0xf67400), c(0x1d99f3), c(0x8e44ad)] },
        "breeze-light" => Pal { app_bg: c(0xfcfcfc), sidebar_bg: c(0xeff0f1), rail_bg: c(0xdde0e3), user_bar_bg: c(0xdde0e3), input_bg: c(0xffffff), hover_bg: c(0xe3e5e7), selected_bg: c(0xd0d4d8), text: c(0x3b3f42), text_muted: c(0x7f8c8d), text_strong: c(0x141618), accent: c(0x3daee9), link: c(0x2980b9), green: c(0x27ae60), red: c(0xda4453), on_color: c(0x141618), avatars: [c(0x2980b9), c(0x8e44ad), c(0x27ae60), c(0xc98a00), c(0xda4453), c(0xf67400), c(0x1aa3b0), c(0x9b59b6)] },
        "catppuccin-mocha" => Pal { app_bg: c(0x1e1e2e), sidebar_bg: c(0x181825), rail_bg: c(0x11111b), user_bar_bg: c(0x11111b), input_bg: c(0x313244), hover_bg: c(0x313244), selected_bg: c(0x45475a), text: c(0xbac2de), text_muted: c(0x7f849c), text_strong: c(0xcdd6f4), accent: c(0xcba6f7), link: c(0x89b4fa), green: c(0xa6e3a1), red: c(0xf38ba8), on_color: c(0x11111b), avatars: [c(0x89b4fa), c(0xf5c2e7), c(0xa6e3a1), c(0xf9e2af), c(0xf38ba8), c(0xfab387), c(0x74c7ec), c(0xcba6f7)] },
        "catppuccin-latte" => Pal { app_bg: c(0xeff1f5), sidebar_bg: c(0xe6e9ef), rail_bg: c(0xdce0e8), user_bar_bg: c(0xdce0e8), input_bg: c(0xffffff), hover_bg: c(0xccd0da), selected_bg: c(0xbcc0cc), text: c(0x5c5f77), text_muted: c(0x8c8fa1), text_strong: c(0x4c4f69), accent: c(0x8839ef), link: c(0x1e66f5), green: c(0x40a02b), red: c(0xd20f39), on_color: c(0xeff1f5), avatars: [c(0x1e66f5), c(0xea76cb), c(0x40a02b), c(0xdf8e1d), c(0xd20f39), c(0xfe640b), c(0x209fb5), c(0x8839ef)] },
        "adwaita-dark" => Pal { app_bg: c(0x242424), sidebar_bg: c(0x2e2e2e), rail_bg: c(0x1e1e1e), user_bar_bg: c(0x1e1e1e), input_bg: c(0x383838), hover_bg: c(0x3d3d3d), selected_bg: c(0x4a4a4a), text: c(0xcdcdcd), text_muted: c(0x9a9996), text_strong: c(0xffffff), accent: c(0x3584e4), link: c(0x78aeed), green: c(0x2ec27e), red: c(0xe01b24), on_color: c(0xffffff), avatars: [c(0x3584e4), c(0x9141ac), c(0x2ec27e), c(0xe5a50a), c(0xe01b24), c(0xff7800), c(0x1c71d8), c(0x613583)] },
        "arc-dark" => Pal { app_bg: c(0x383c4a), sidebar_bg: c(0x2f343f), rail_bg: c(0x272b35), user_bar_bg: c(0x272b35), input_bg: c(0x2f343f), hover_bg: c(0x3b414d), selected_bg: c(0x434a58), text: c(0xd3dae3), text_muted: c(0x7b8496), text_strong: c(0xf5f7fa), accent: c(0x5294e2), link: c(0x5294e2), green: c(0x6abf69), red: c(0xbf616a), on_color: c(0x1e222a), avatars: [c(0x5294e2), c(0xb48ead), c(0xa3be8c), c(0xebcb8b), c(0xbf616a), c(0xd08770), c(0x88c0d0), c(0x8f6aa8)] },
        "dracula" => Pal { app_bg: c(0x282a36), sidebar_bg: c(0x21222c), rail_bg: c(0x191a21), user_bar_bg: c(0x191a21), input_bg: c(0x44475a), hover_bg: c(0x44475a), selected_bg: c(0x6272a4), text: c(0xf8f8f2), text_muted: c(0x6272a4), text_strong: c(0xffffff), accent: c(0xbd93f9), link: c(0x8be9fd), green: c(0x50fa7b), red: c(0xff5555), on_color: c(0x282a36), avatars: [c(0xbd93f9), c(0xff79c6), c(0x50fa7b), c(0xf1fa8c), c(0xff5555), c(0xffb86c), c(0x8be9fd), c(0x6272a4)] },
        "nord" => Pal { app_bg: c(0x2e3440), sidebar_bg: c(0x272c36), rail_bg: c(0x242933), user_bar_bg: c(0x242933), input_bg: c(0x3b4252), hover_bg: c(0x3b4252), selected_bg: c(0x434c5e), text: c(0xd8dee9), text_muted: c(0x7b88a1), text_strong: c(0xeceff4), accent: c(0x88c0d0), link: c(0x81a1c1), green: c(0xa3be8c), red: c(0xbf616a), on_color: c(0x2e3440), avatars: [c(0x88c0d0), c(0xb48ead), c(0xa3be8c), c(0xebcb8b), c(0xbf616a), c(0xd08770), c(0x5e81ac), c(0x81a1c1)] },
        "gruvbox-dark" => Pal { app_bg: c(0x282828), sidebar_bg: c(0x1d2021), rail_bg: c(0x1d2021), user_bar_bg: c(0x1d2021), input_bg: c(0x3c3836), hover_bg: c(0x3c3836), selected_bg: c(0x504945), text: c(0xebdbb2), text_muted: c(0x928374), text_strong: c(0xfbf1c7), accent: c(0x83a598), link: c(0x83a598), green: c(0xb8bb26), red: c(0xfb4934), on_color: c(0x282828), avatars: [c(0x83a598), c(0xd3869b), c(0xb8bb26), c(0xfabd2f), c(0xfb4934), c(0xfe8019), c(0x8ec07c), c(0xd65d0e)] },
        "adwaita-light" => Pal { app_bg: c(0xffffff), sidebar_bg: c(0xf6f5f4), rail_bg: c(0xe1dedb), user_bar_bg: c(0xe1dedb), input_bg: c(0xffffff), hover_bg: c(0xe0ddda), selected_bg: c(0xd5d1cd), text: c(0x2e3436), text_muted: c(0x6e6e73), text_strong: c(0x1e1e1e), accent: c(0x3584e4), link: c(0x1c71d8), green: c(0x26a269), red: c(0xc01c28), on_color: c(0xffffff), avatars: [c(0x3584e4), c(0x9141ac), c(0x26a269), c(0xe5a50a), c(0xc01c28), c(0xff7800), c(0x1c71d8), c(0x613583)] },
        "arc-light" => Pal { app_bg: c(0xf5f6f7), sidebar_bg: c(0xe3e5e8), rail_bg: c(0xd7dadf), user_bar_bg: c(0xd7dadf), input_bg: c(0xffffff), hover_bg: c(0xd3d7dd), selected_bg: c(0xc6cbd3), text: c(0x2e3440), text_muted: c(0x6b7280), text_strong: c(0x111418), accent: c(0x5294e2), link: c(0x5294e2), green: c(0x4a8a3c), red: c(0xd1343d), on_color: c(0xffffff), avatars: [c(0x5294e2), c(0x8e6fb3), c(0x4a8a3c), c(0xc18a1f), c(0xd1343d), c(0xd0703e), c(0x3f8fa0), c(0x7b5ea7)] },
        "solarized-light" => Pal { app_bg: c(0xfdf6e3), sidebar_bg: c(0xeee8d5), rail_bg: c(0xe4ddc8), user_bar_bg: c(0xe4ddc8), input_bg: c(0xfdf6e3), hover_bg: c(0xe6dfca), selected_bg: c(0xddd6c1), text: c(0x586e75), text_muted: c(0x93a1a1), text_strong: c(0x073642), accent: c(0x268bd2), link: c(0x268bd2), green: c(0x859900), red: c(0xdc322f), on_color: c(0xfdf6e3), avatars: [c(0x268bd2), c(0x6c71c4), c(0x859900), c(0xb58900), c(0xdc322f), c(0xcb4b16), c(0x2aa198), c(0xd33682)] },
        "gruvbox-light" => Pal { app_bg: c(0xfbf1c7), sidebar_bg: c(0xf2e5bc), rail_bg: c(0xebdbb2), user_bar_bg: c(0xebdbb2), input_bg: c(0xfdf9e8), hover_bg: c(0xebdbb2), selected_bg: c(0xd5c4a1), text: c(0x3c3836), text_muted: c(0x7c6f64), text_strong: c(0x282828), accent: c(0x076678), link: c(0x076678), green: c(0x79740e), red: c(0x9d0006), on_color: c(0xfbf1c7), avatars: [c(0x076678), c(0x8f3f71), c(0x79740e), c(0xb57614), c(0x9d0006), c(0xaf3a03), c(0x427b58), c(0xb16286)] },
        "ayu-light" => Pal { app_bg: c(0xfafafa), sidebar_bg: c(0xf3f4f5), rail_bg: c(0xeceef0), user_bar_bg: c(0xeceef0), input_bg: c(0xffffff), hover_bg: c(0xe7e8e9), selected_bg: c(0xdcdde0), text: c(0x5c6166), text_muted: c(0x8a9199), text_strong: c(0x1a1f29), accent: c(0xff9940), link: c(0x399ee6), green: c(0x86b300), red: c(0xf07171), on_color: c(0xffffff), avatars: [c(0x399ee6), c(0xa37acc), c(0x86b300), c(0xf2ae49), c(0xf07171), c(0xff9940), c(0x4cbf99), c(0xfa8d3e)] },
        _ => palette("breeze-dark"),
    }
}

/// The themes in the order the menu shows them: id, name, and whether they are dark or light.
pub const THEMES: &[(&str, &str, &str)] = &[
    ("breeze-dark", "Breeze Dark", "dark"),
    ("breeze-light", "Breeze Light", "light"),
    ("catppuccin-mocha", "Catppuccin Mocha", "dark"),
    ("catppuccin-latte", "Catppuccin Latte", "light"),
    ("adwaita-dark", "Adwaita Dark", "dark"),
    ("arc-dark", "Arc Dark", "dark"),
    ("dracula", "Dracula", "dark"),
    ("nord", "Nord", "dark"),
    ("gruvbox-dark", "Gruvbox Dark", "dark"),
    ("adwaita-light", "Adwaita Light", "light"),
    ("arc-light", "Arc Light", "light"),
    ("solarized-light", "Solarized Light", "light"),
    ("gruvbox-light", "Gruvbox Light", "light"),
    ("ayu-light", "Ayu Light", "light"),
];

/// The ids of every theme.
pub const IDS: &[&str] = &[
    "breeze-dark", "breeze-light", "catppuccin-mocha", "catppuccin-latte", "adwaita-dark", "arc-dark",
    "dracula", "nord", "gruvbox-dark", "adwaita-light", "arc-light", "solarized-light", "gruvbox-light",
    "ayu-light",
];

/// The iced theme for a palette, so widgets without their own colours still match the app.
pub fn iced_theme(id: &str) -> iced::Theme {
    let p = palette(id);
    iced::Theme::custom(
        id.to_string(),
        iced::theme::Palette {
            background: p.app_bg,
            text: p.text,
            primary: p.accent,
            success: p.green,
            warning: p.avatars[3],
            danger: p.red,
        },
    )
}
