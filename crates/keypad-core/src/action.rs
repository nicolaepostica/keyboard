//! Media actions a function key can trigger.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    BrightnessDown,
    BrightnessUp,
    KbdIllumDown,
    KbdIllumUp,
    PrevTrack,
    PlayPause,
    NextTrack,
    Mute,
    VolumeDown,
    VolumeUp,
}

impl Action {
    pub const ALL: [Action; 10] = [
        Action::BrightnessDown,
        Action::BrightnessUp,
        Action::KbdIllumDown,
        Action::KbdIllumUp,
        Action::PrevTrack,
        Action::PlayPause,
        Action::NextTrack,
        Action::Mute,
        Action::VolumeDown,
        Action::VolumeUp,
    ];

    /// Stable name used in config files and commands.
    pub fn name(self) -> &'static str {
        match self {
            Action::BrightnessDown => "brightness_down",
            Action::BrightnessUp => "brightness_up",
            Action::KbdIllumDown => "kbd_illum_down",
            Action::KbdIllumUp => "kbd_illum_up",
            Action::PrevTrack => "prev_track",
            Action::PlayPause => "play_pause",
            Action::NextTrack => "next_track",
            Action::Mute => "mute",
            Action::VolumeDown => "volume_down",
            Action::VolumeUp => "volume_up",
        }
    }

    /// Human label for the UI.
    pub fn label(self) -> &'static str {
        match self {
            Action::BrightnessDown => "Brightness down",
            Action::BrightnessUp => "Brightness up",
            Action::KbdIllumDown => "Keyboard light down",
            Action::KbdIllumUp => "Keyboard light up",
            Action::PrevTrack => "Previous track",
            Action::PlayPause => "Play / pause",
            Action::NextTrack => "Next track",
            Action::Mute => "Mute",
            Action::VolumeDown => "Volume down",
            Action::VolumeUp => "Volume up",
        }
    }

    pub fn from_name(s: &str) -> Option<Action> {
        Action::ALL.into_iter().find(|a| a.name() == s)
    }
}
