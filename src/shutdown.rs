use crate::log::Log;
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

pub const DELAY_S: u32 = 60;
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Default)]
pub struct Shutdown {
    pub on_disconnect: Arc<AtomicBool>,
    pub after_runs: Arc<AtomicU32>,
}

fn limit_reached(runs: u32, limit: u32) -> bool {
    limit > 0 && runs >= limit
}

impl Shutdown {
    pub fn runs_done(&self, runs: u32) -> bool {
        limit_reached(runs, self.after_runs.load(Ordering::Relaxed))
    }
}

pub fn begin(reason: &str, log: &Log) {
    let delay = DELAY_S.to_string();
    let started = Command::new("shutdown").args(["/s", "/t", &delay]).creation_flags(CREATE_NO_WINDOW).status().is_ok_and(|status| status.success());
    if started {
        log.warn(format!("{reason} - shutting down in {DELAY_S}s. Run \"shutdown /a\" to cancel"));
    } else {
        log.error(format!("{reason} - couldn't shut down"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_means_no_limit() {
        assert!(!limit_reached(500, 0));
        assert!(!limit_reached(4, 5));
        assert!(limit_reached(5, 5));
        assert!(limit_reached(6, 5));
    }
}
