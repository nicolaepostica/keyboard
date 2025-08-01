//! Platform-neutral key identities.

/// A modifier key, left and right kept apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Modifier {
    LCtrl,
    RCtrl,
    LAlt,
    RAlt,
    LShift,
    RShift,
    LSuper,
    RSuper,
}

/// A key as the decider sees it. Platforms translate their native codes into this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// Function key F1..=F12. Values outside that range are treated as "other".
    F(u8),
    /// Letter `'a'..='z'` (lowercase).
    Letter(char),
    Mod(Modifier),
    Other,
}
