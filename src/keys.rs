#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Key {
    pub name: &'static str,
    pub vk: u16,
    pub extended: bool,
}

const fn key(name: &'static str, vk: u16, extended: bool) -> Key {
    Key { name, vk, extended }
}

const NAMED: [Key; 66] = [
    key("Space", 0x20, false),
    key("Enter", 0x0D, false),
    key("Tab", 0x09, false),
    key("Escape", 0x1B, false),
    key("Backspace", 0x08, false),
    key("CapsLock", 0x14, false),
    key("LShift", 0xA0, false),
    key("RShift", 0xA1, false),
    key("LControl", 0xA2, false),
    key("RControl", 0xA3, true),
    key("LAlt", 0xA4, false),
    key("RAlt", 0xA5, true),
    key("LWin", 0x5B, true),
    key("RWin", 0x5C, true),
    key("Left", 0x25, true),
    key("Up", 0x26, true),
    key("Right", 0x27, true),
    key("Down", 0x28, true),
    key("Home", 0x24, true),
    key("End", 0x23, true),
    key("PgUp", 0x21, true),
    key("PgDn", 0x22, true),
    key("Insert", 0x2D, true),
    key("Delete", 0x2E, true),
    key("F1", 0x70, false),
    key("F2", 0x71, false),
    key("F3", 0x72, false),
    key("F4", 0x73, false),
    key("F5", 0x74, false),
    key("F6", 0x75, false),
    key("F7", 0x76, false),
    key("F8", 0x77, false),
    key("F9", 0x78, false),
    key("F10", 0x79, false),
    key("F11", 0x7A, false),
    key("F12", 0x7B, false),
    key("Numpad0", 0x60, false),
    key("Numpad1", 0x61, false),
    key("Numpad2", 0x62, false),
    key("Numpad3", 0x63, false),
    key("Numpad4", 0x64, false),
    key("Numpad5", 0x65, false),
    key("Numpad6", 0x66, false),
    key("Numpad7", 0x67, false),
    key("Numpad8", 0x68, false),
    key("Numpad9", 0x69, false),
    key("NumpadMult", 0x6A, false),
    key("NumpadAdd", 0x6B, false),
    key("NumpadSub", 0x6D, false),
    key("NumpadDot", 0x6E, false),
    key("NumpadDiv", 0x6F, true),
    key(";", 0xBA, false),
    key("=", 0xBB, false),
    key(",", 0xBC, false),
    key("-", 0xBD, false),
    key(".", 0xBE, false),
    key("/", 0xBF, false),
    key("`", 0xC0, false),
    key("[", 0xDB, false),
    key("\\", 0xDC, false),
    key("]", 0xDD, false),
    key("'", 0xDE, false),
    key("Shift", 0x10, false),
    key("Control", 0x11, false),
    key("Ctrl", 0x11, false),
    key("Alt", 0x12, false),
];

const SHARED_MODIFIERS: usize = 4;

const LETTERS: &str = "abcdefghijklmnopqrstuvwxyz";
const DIGITS: &str = "0123456789";

fn single_char(name: &str) -> Option<Key> {
    if name.len() != 1 {
        return None;
    }
    let lower = name.to_ascii_lowercase();
    if let Some(at) = LETTERS.find(&lower) {
        return Some(key(&LETTERS[at..at + 1], 0x41 + at as u16, false));
    }
    DIGITS.find(&lower).map(|at| key(&DIGITS[at..at + 1], 0x30 + at as u16, false))
}

pub fn by_name(name: &str) -> Option<Key> {
    single_char(name).or_else(|| NAMED.iter().find(|known| known.name.eq_ignore_ascii_case(name)).copied())
}

pub fn recordable() -> Vec<Key> {
    let chars = LETTERS.chars().chain(DIGITS.chars()).filter_map(|c| single_char(&c.to_string()));
    chars.chain(NAMED[..NAMED.len() - SHARED_MODIFIERS].iter().copied()).collect()
}

pub fn is_modifier(name: &str) -> bool {
    by_name(name).is_some_and(|known| matches!(known.vk, 0x10..=0x12 | 0xA0..=0xA5 | 0x5B | 0x5C))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_match_autohotkey() {
        assert_eq!(by_name("w").map(|k| k.vk), Some(0x57));
        assert_eq!(by_name("W").map(|k| k.name), Some("w"));
        assert_eq!(by_name("7").map(|k| k.vk), Some(0x37));
        assert_eq!(by_name("Right"), Some(key("Right", 0x27, true)));
        assert_eq!(by_name("lshift").map(|k| k.vk), Some(0xA0));
        assert_eq!(by_name("NotAKey"), None);
    }

    #[test]
    fn recordable_keys_are_unique_and_skip_shared_modifiers() {
        let keys = recordable();
        assert!(keys.iter().all(|k| !matches!(k.vk, 0x10..=0x12)));
        let mut vks: Vec<u16> = keys.iter().map(|k| k.vk).collect();
        vks.sort();
        vks.dedup();
        assert_eq!(vks.len(), keys.len());
        assert!(keys.iter().any(|k| k.name == "z"));
    }

    #[test]
    fn modifiers_are_known() {
        assert!(is_modifier("LControl") && is_modifier("Shift") && is_modifier("RWin"));
        assert!(!is_modifier("i") && !is_modifier("Right"));
    }
}
