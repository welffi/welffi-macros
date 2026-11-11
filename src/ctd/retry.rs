use super::assets::EXTRA_TOLERANCE;
use super::{BOX, FOCUS_MS, Outcome, Run};
use crate::playback;
use crate::roblox;
use std::time::Duration;

const RETRY_AFTER: Duration = Duration::from_secs(10);

pub(super) fn retry_due(quiet: Duration) -> bool {
    quiet >= RETRY_AFTER
}

impl Run {
    pub(super) fn cancel_showing(&self, index: usize) -> bool {
        let client = &self.clients[index];
        let Some(retry) = &self.assets.retry else {
            return false;
        };
        retry_due(client.quiet_since.elapsed()) && self.search(index, &retry.cancel, EXTRA_TOLERANCE).is_some()
    }

    pub(super) fn retry_restart(&mut self, index: usize) -> Outcome {
        let tag = self.tag(&self.clients[index]);
        let window = self.clients[index].window;
        self.log.warn(format!("{tag}Cancel still showing - retrying restart"));
        self.set_state(index, "restart retry".to_string());
        if !roblox::is_foreground(window) {
            roblox::activate(window);
            if !self.cancel.nap(FOCUS_MS) {
                return Outcome::Stop;
            }
        }
        let Some(retry) = &self.assets.retry else {
            return Outcome::Waiting;
        };
        let cancel = self.cancel.clone();
        let result = playback::play(&retry.recording, window, BOX, &self.log, &|| cancel.is_set());
        if self.cancel.is_set() {
            return Outcome::Stop;
        }
        if let Err(message) = result {
            self.log.warn(format!("{tag}restart retry - {message}"));
        }
        self.set_step(index, 0);
        Outcome::Played
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cancel_button_is_checked_ten_seconds_after_each_restart() {
        assert!(!retry_due(Duration::from_secs(9)));
        assert!(retry_due(Duration::from_secs(10)));
    }
}
