use crate::disconnect;
use crate::log::Log;
use crate::recording::{self, Recording};
use crate::store;
use crate::variant::Variant;
use crate::vision::{self, Template};

const TEMPLATES: &str = "ctd";
const TOILET_CITY: &str = "toilet_city";
const UNITS_LOBBY: &str = "units_lobby";
const ENTER_TOILET_CITY: &str = "enter_toilet_city";
const RESULTS_CANCEL: &str = "results_cancel";
const RESTART_RETRY: &str = "restart_retry";
pub(super) const EXTRA_TOLERANCE: i32 = 60;

pub(super) struct Step {
    pub(super) name: &'static str,
    pub(super) templates: &'static [&'static str],
    pub(super) tolerance: i32,
    pub(super) recording: &'static str,
    pub(super) label: &'static str,
    pub(super) hand_off_ms: Option<u64>,
}

pub(super) const STEPS: [Step; 2] = [
    Step { name: "start_game", templates: &["start_game"], tolerance: 60, recording: "setup_camera", label: "setup camera", hand_off_ms: None },
    Step { name: "results", templates: &["results_lost", "results_won"], tolerance: 60, recording: "restart", label: "restart", hand_off_ms: Some(1500) },
];

pub(super) struct Lobby {
    pub(super) toilet_city: Vec<Template>,
    pub(super) units_lobby: Vec<Template>,
    pub(super) enter: Recording,
}

pub(super) struct Retry {
    pub(super) cancel: Vec<Template>,
    pub(super) recording: Recording,
}

pub(super) struct Assets {
    pub(super) templates: Vec<Vec<Template>>,
    pub(super) recordings: Vec<Recording>,
    pub(super) disconnected: Option<Template>,
    pub(super) lobby: Option<Lobby>,
    pub(super) retry: Option<Retry>,
}

fn load_lobby(variant: Variant, log: &Log) -> Option<Lobby> {
    let toilet_city = vision::load(&store::template_path(TEMPLATES, TOILET_CITY));
    let enter = recording::load(&store::recording_path(variant.recordings(), ENTER_TOILET_CITY));
    let units_lobby = vision::load(&store::template_path(TEMPLATES, UNITS_LOBBY));
    match (toilet_city, enter) {
        (Some(toilet_city), Some(enter)) => Some(Lobby { toilet_city: vec![toilet_city], units_lobby: units_lobby.into_iter().collect(), enter }),
        _ => {
            log.warn(format!("Missing {TOILET_CITY}.png or {ENTER_TOILET_CITY}.txt - lobby recovery is off"));
            None
        }
    }
}

fn load_retry(variant: Variant, log: &Log) -> Option<Retry> {
    let cancel = vision::load(&store::template_path(TEMPLATES, RESULTS_CANCEL));
    let recording = recording::load(&store::recording_path(variant.recordings(), RESTART_RETRY));
    match (cancel, recording) {
        (Some(cancel), Some(recording)) => Some(Retry { cancel: vec![cancel], recording }),
        _ => {
            log.info(format!("No {RESULTS_CANCEL}.png or {RESTART_RETRY}.txt - restart retry is off"));
            None
        }
    }
}

pub(super) fn load_assets(variant: Variant, log: &Log) -> Option<Assets> {
    let mut missing = Vec::new();
    let mut assets = Assets { templates: Vec::new(), recordings: Vec::new(), disconnected: disconnect::load(), lobby: load_lobby(variant, log), retry: load_retry(variant, log) };
    for step in &STEPS {
        match recording::load(&store::recording_path(variant.recordings(), step.recording)) {
            Some(take) => {
                log.info(format!("{} - {} frames, {:.1}s", step.recording, take.frames.len(), take.duration() / 1000.0));
                assets.recordings.push(take);
            }
            None => missing.push(format!("{}.txt", step.recording)),
        }
        let mut found = Vec::new();
        for name in step.templates {
            match vision::load(&store::template_path(TEMPLATES, name)) {
                Some(template) => found.push(template),
                None => missing.push(format!("{name}.png")),
            }
        }
        assets.templates.push(found);
    }
    if !missing.is_empty() {
        log.warn(format!("Missing files: {}", missing.join(", ")));
        return None;
    }
    Some(assets)
}
