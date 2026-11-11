#[derive(Clone, Copy, Default, PartialEq)]
pub enum Variant {
    #[default]
    Easy,
    EasyClear,
}

impl Variant {
    pub fn id(self) -> &'static str {
        match self {
            Variant::Easy => "ctd_easy",
            Variant::EasyClear => "ctd_easy_clear",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Variant::Easy => "Easy",
            Variant::EasyClear => "Easy Clear",
        }
    }

    pub fn recordings(self) -> &'static str {
        match self {
            Variant::Easy => "ctd/easy",
            Variant::EasyClear => "ctd/easy_clear",
        }
    }

    pub fn every_window(self) -> bool {
        self == Variant::Easy
    }

    pub fn cuts_setup_on_results(self) -> bool {
        self == Variant::EasyClear
    }
}
