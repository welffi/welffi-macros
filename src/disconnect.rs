use crate::roblox::{self, Hwnd};
use crate::store;
use crate::vision::{self, Template};
use std::time::{Duration, Instant};

const TEMPLATES: &str = "global";
const TEMPLATE: &str = "disconnected";
const TOLERANCE: i32 = 60;
const CHECK_EVERY: Duration = Duration::from_secs(2);

pub fn load() -> Option<Template> {
    vision::load(&store::template_path(TEMPLATES, TEMPLATE))
}

pub fn due(next_check: &mut Instant) -> bool {
    let now = Instant::now();
    if now < *next_check {
        return false;
    }
    *next_check = now + CHECK_EVERY;
    true
}

pub fn showing(window: Hwnd, template: &Template, leniency: i32) -> bool {
    let tolerance = vision::tolerance(TOLERANCE, leniency);
    roblox::client_rect(window).and_then(|area| vision::find(area, std::slice::from_ref(template), tolerance).ok().flatten()).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_once_every_two_seconds() {
        let mut next_check = Instant::now();
        assert!(due(&mut next_check));
        assert!(!due(&mut next_check));
        next_check = Instant::now() - Duration::from_millis(1);
        assert!(due(&mut next_check));
    }
}
