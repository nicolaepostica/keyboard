//! Colours shared by the app's icons and window (placeholder until assets/theme.toml, G10).

pub struct Palette {
    pub accent: [u8; 4],
    pub on_accent: [u8; 4],
    pub muted: [u8; 4],
}

pub fn palette() -> Palette {
    Palette {
        accent: [0x2f, 0x9e, 0x8f, 0xff],
        on_accent: [0xff, 0xff, 0xff, 0xff],
        muted: [0x8a, 0x8f, 0x98, 0xff],
    }
}
