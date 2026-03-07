/// Semantic icon set gated by `use_nerd_fonts` config flag.
///
/// Two const instances exist: [`UNICODE`] (safe fallback) and [`NERD`]
/// (Nerd Font glyphs).  Use [`icons()`] to pick the right one at runtime.
pub struct Icons {
    pub room: &'static str,
    pub dm: &'static str,
    pub space: &'static str,
    pub expand: &'static str,
    pub collapse: &'static str,
    pub unverified: &'static str,
    pub power_owner: &'static str,
    pub power_admin: &'static str,
    pub power_mod: &'static str,
    pub power_voice: &'static str,
    pub power_none: &'static str,
    pub selected: &'static str,
    pub unselected: &'static str,
    pub arrow_left: &'static str,
    pub arrow_right: &'static str,
    pub voice: &'static str,
    pub home: &'static str,
    pub participant: &'static str,
}

pub const UNICODE: Icons = Icons {
    room: "#",
    dm: "@",
    space: "\u{2261}",
    expand: "\u{25BC}",
    collapse: "\u{25B6}",
    unverified: "\u{26A0}",
    power_owner: "~",
    power_admin: "&",
    power_mod: "@",
    power_voice: "+",
    power_none: " ",
    selected: "\u{25C8}",
    unselected: "\u{25C7}",
    arrow_left: "\u{25C2}",
    arrow_right: "\u{25B8}",
    voice: "\u{25C9}",
    home: "\u{2302}",
    participant: "\u{25B6}",
};

pub const NERD: Icons = Icons {
    room: "\u{f4ad}",
    dm: "\u{f007}",
    space: "\u{f07c}",
    expand: "\u{f0d7}",
    collapse: "\u{f0da}",
    unverified: "\u{f071}",
    power_owner: "\u{f521}",
    power_admin: "\u{f132}",
    power_mod: "\u{f005}",
    power_voice: "\u{f130}",
    power_none: " ",
    selected: "\u{f192}",
    unselected: "\u{f10c}",
    arrow_left: "\u{f104}",
    arrow_right: "\u{f105}",
    voice: "\u{f130}",
    home: "\u{f015}",
    participant: "\u{f007}",
};

pub fn icons(use_nerd_fonts: bool) -> &'static Icons {
    if use_nerd_fonts { &NERD } else { &UNICODE }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_fields_non_empty() {
        let i = &UNICODE;
        assert!(!i.room.is_empty());
        assert!(!i.dm.is_empty());
        assert!(!i.space.is_empty());
        assert!(!i.expand.is_empty());
        assert!(!i.collapse.is_empty());
        assert!(!i.unverified.is_empty());
        assert!(!i.power_owner.is_empty());
        assert!(!i.power_admin.is_empty());
        assert!(!i.power_mod.is_empty());
        assert!(!i.power_voice.is_empty());
        assert!(!i.selected.is_empty());
        assert!(!i.unselected.is_empty());
        assert!(!i.arrow_left.is_empty());
        assert!(!i.arrow_right.is_empty());
        assert!(!i.voice.is_empty());
        assert!(!i.home.is_empty());
        assert!(!i.participant.is_empty());
    }

    #[test]
    fn nerd_fields_non_empty() {
        let i = &NERD;
        assert!(!i.room.is_empty());
        assert!(!i.dm.is_empty());
        assert!(!i.space.is_empty());
        assert!(!i.expand.is_empty());
        assert!(!i.collapse.is_empty());
        assert!(!i.unverified.is_empty());
        assert!(!i.power_owner.is_empty());
        assert!(!i.power_admin.is_empty());
        assert!(!i.power_mod.is_empty());
        assert!(!i.power_voice.is_empty());
        assert!(!i.selected.is_empty());
        assert!(!i.unselected.is_empty());
        assert!(!i.arrow_left.is_empty());
        assert!(!i.arrow_right.is_empty());
        assert!(!i.voice.is_empty());
        assert!(!i.home.is_empty());
        assert!(!i.participant.is_empty());
    }

    #[test]
    fn sets_differ() {
        assert_ne!(UNICODE.room, NERD.room);
        assert_ne!(UNICODE.selected, NERD.selected);
    }

    #[test]
    fn selector_returns_correct_set() {
        let u = icons(false);
        let n = icons(true);
        assert_eq!(u.room, UNICODE.room);
        assert_eq!(n.room, NERD.room);
    }
}
