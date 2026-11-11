use super::assets::EXTRA_TOLERANCE;
use super::{BOX, FOCUS_MS, Outcome, RESULTS_STEP, Run};
use crate::fit;
use crate::input;
use crate::launch;
use crate::playback;
use crate::roblox::{self, Hwnd};
use std::time::{Duration, Instant};

const LOBBY_AFTER: Duration = Duration::from_secs(30);
const UNITS_AFTER: Duration = Duration::from_secs(10);
const REJOIN_AFTER: Duration = Duration::from_secs(120);
const NEW_WINDOW_WAIT: Duration = Duration::from_secs(120);
const CLOSE_WAIT: Duration = Duration::from_secs(5);
const RESET_KEYS: [&str; 3] = ["Escape", "r", "Enter"];
const KEY_HOLD_MS: u64 = 40;
const KEY_GAP_MS: u64 = 150;
const RESET_GAP_MS: u64 = 3500;
const RETRY_MS: u64 = 30_000;
const SETTLE_MS: u64 = 5000;
const FIT_TRIES: usize = 5;
const FIT_GAP_MS: u64 = 2000;

fn lobby_due(step: usize, quiet: Duration, units_seen: impl FnOnce() -> bool) -> bool {
    (step != RESULTS_STEP && quiet >= LOBBY_AFTER) || (quiet >= UNITS_AFTER && units_seen())
}

fn rejoin_due(recovering: Duration) -> bool {
    recovering >= REJOIN_AFTER
}

impl Run {
    pub(super) fn in_lobby(&self, index: usize) -> bool {
        let client = &self.clients[index];
        self.assets.lobby.is_some() && lobby_due(client.step, client.quiet_since.elapsed(), || self.sees_units_lobby(index)) && self.look(index, RESULTS_STEP).is_none()
    }

    fn sees_units_lobby(&self, index: usize) -> bool {
        self.assets.retry.is_none() && self.assets.lobby.as_ref().is_some_and(|lobby| self.search(index, &lobby.units_lobby, EXTRA_TOLERANCE).is_some())
    }

    pub(super) fn begin_recovery(&mut self, index: usize) {
        let tag = self.tag(&self.clients[index]);
        self.log.warn(format!("{tag}No start_game or results - resetting until toilet city shows"));
        self.clients[index].recovering = Some(Instant::now());
        self.set_state(index, "lobby reset".to_string());
    }

    fn end_recovery(&mut self, index: usize, step: usize) {
        self.clients[index].recovering = None;
        self.set_step(index, step);
    }

    pub(super) fn recover(&mut self, index: usize) -> Outcome {
        if self.look(index, 0).is_some() {
            self.end_recovery(index, 0);
            return Outcome::Waiting;
        }
        if self.look(index, RESULTS_STEP).is_some() {
            self.end_recovery(index, RESULTS_STEP);
            return Outcome::Waiting;
        }
        if self.sees_toilet_city(index) {
            return self.enter_toilet_city(index);
        }
        let started = self.clients[index].recovering.unwrap_or_else(Instant::now);
        if !rejoin_due(started.elapsed()) {
            return self.reset_character(index);
        }
        if self.target.is_some() {
            return self.rejoin(index);
        }
        self.log.warn("Toilet city not found for 2 minutes - pick a private server to rejoin");
        self.clients[index].recovering = Some(Instant::now());
        Outcome::Waiting
    }

    fn sees_toilet_city(&self, index: usize) -> bool {
        self.assets.lobby.as_ref().is_some_and(|lobby| self.search(index, &lobby.toilet_city, EXTRA_TOLERANCE).is_some())
    }

    fn enter_toilet_city(&mut self, index: usize) -> Outcome {
        let tag = self.tag(&self.clients[index]);
        let window = self.clients[index].window;
        self.log.ok(format!("{tag}toilet city found"));
        self.set_state(index, "enter toilet city".to_string());
        if !roblox::is_foreground(window) {
            roblox::activate(window);
            if !self.cancel.nap(FOCUS_MS) {
                return Outcome::Stop;
            }
        }
        let Some(lobby) = &self.assets.lobby else {
            self.end_recovery(index, 0);
            return Outcome::Waiting;
        };
        let cancel = self.cancel.clone();
        let result = playback::play(&lobby.enter, window, BOX, &self.log, &|| cancel.is_set());
        if self.cancel.is_set() {
            return Outcome::Stop;
        }
        if let Err(message) = result {
            self.log.warn(format!("{tag}enter toilet city - {message}"));
        }
        self.end_recovery(index, 0);
        Outcome::Played
    }

    fn reset_character(&mut self, index: usize) -> Outcome {
        let window = self.clients[index].window;
        if !roblox::is_foreground(window) {
            roblox::activate(window);
            if !self.cancel.nap(FOCUS_MS) {
                return Outcome::Stop;
            }
        }
        for key in RESET_KEYS {
            input::key(key, true);
            let held = self.cancel.nap(KEY_HOLD_MS);
            input::key(key, false);
            if !held || !self.cancel.nap(KEY_GAP_MS) {
                return Outcome::Stop;
            }
        }
        if self.cancel.nap(RESET_GAP_MS) { Outcome::Waiting } else { Outcome::Stop }
    }

    pub(super) fn rejoin(&mut self, index: usize) -> Outcome {
        let Some(target) = self.target.clone() else {
            return Outcome::Waiting;
        };
        let tag = self.tag(&self.clients[index]);
        let old = self.clients[index].window;
        self.log.warn(format!("{tag}Rejoining as {}", target.account.name));
        self.set_state(index, "rejoining".to_string());
        roblox::close(old);
        roblox::wait_for(CLOSE_WAIT, || !roblox::exists(old));
        let before = roblox::find_all();
        let window = loop {
            match launch::join_private(&target.account, &target.link, &self.log) {
                Ok(()) => match self.wait_for_new_window(&before) {
                    Some(window) => break window,
                    None if self.cancel.is_set() => return Outcome::Stop,
                    None => self.log.warn(format!("{tag}Roblox didn't open - trying again")),
                },
                Err(message) => {
                    self.log.error(format!("{tag}Rejoin failed - {message}"));
                    if !self.cancel.nap(RETRY_MS) {
                        return Outcome::Stop;
                    }
                }
            }
        };
        if !self.cancel.nap(SETTLE_MS) || !self.fit_new_window(window, &tag) {
            return Outcome::Stop;
        }
        self.log.ok(format!("{tag}Rejoined"));
        self.clients[index].window = window;
        self.shown = Some(window);
        self.set_step(index, 0);
        self.clients[index].recovering = Some(Instant::now());
        self.set_state(index, "lobby reset".to_string());
        Outcome::Played
    }

    fn wait_for_new_window(&self, before: &[Hwnd]) -> Option<Hwnd> {
        let deadline = Instant::now() + NEW_WINDOW_WAIT;
        while Instant::now() < deadline {
            if let Some(window) = roblox::find_all().into_iter().find(|window| !before.contains(window)) {
                return Some(window);
            }
            if !self.cancel.nap(1000) {
                return None;
            }
        }
        None
    }

    fn fit_new_window(&self, window: Hwnd, tag: &str) -> bool {
        for _ in 0..FIT_TRIES {
            if fit::fit(window, Some(BOX)).is_ok() {
                return true;
            }
            if !self.cancel.nap(FIT_GAP_MS) {
                return false;
            }
        }
        self.log.warn(format!("{tag}Couldn't fit the new Roblox window - press Detect + Fit"));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lobby_is_only_guessed_while_waiting_for_start_game() {
        assert!(!lobby_due(0, Duration::from_secs(29), || false));
        assert!(lobby_due(0, Duration::from_secs(30), || false));
        assert!(!lobby_due(RESULTS_STEP, Duration::from_secs(600), || false));
    }

    #[test]
    fn the_units_button_shows_the_lobby_at_any_step_after_a_short_wait() {
        assert!(lobby_due(RESULTS_STEP, Duration::from_secs(10), || true));
        assert!(lobby_due(0, Duration::from_secs(10), || true));
        assert!(!lobby_due(0, Duration::from_secs(9), || true));
    }

    #[test]
    fn a_rejoin_comes_after_two_minutes() {
        assert!(!rejoin_due(Duration::from_secs(119)));
        assert!(rejoin_due(Duration::from_secs(120)));
    }
}
