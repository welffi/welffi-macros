use crate::keys;
use std::path::Path;

const TAIL_PAD_MS: f64 = 600.0;
const OLD_POLL_MS: u32 = 16;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frame {
    pub x: i32,
    pub y: i32,
    pub keys: Vec<String>,
    pub buttons: Vec<String>,
    pub wheel: i32,
    pub look: (i32, i32),
    pub t: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Recording {
    pub poll_ms: u32,
    pub width: i32,
    pub height: i32,
    pub frames: Vec<Frame>,
}

fn words(text: Option<&str>) -> Vec<String> {
    text.unwrap_or("").split(' ').filter(|word| !word.is_empty()).map(str::to_string).collect()
}

fn parse_frame(line: &str, index: usize, poll_ms: u32) -> Option<Frame> {
    let mut parts = line.split(';');
    let (x, y) = parts.next()?.split_once(',')?;
    let keys = words(parts.next());
    let buttons = words(parts.next());
    let wheel = parts.next().filter(|text| !text.is_empty()).map_or(Some(0), |text| text.trim().parse().ok())?;
    let t = parts.next().filter(|text| !text.is_empty()).map_or(Some(index as f64 * poll_ms as f64), |text| text.trim().parse().ok())?;
    let look = match parts.next().and_then(|text| text.split_once(',')) {
        Some((dx, dy)) => (dx.trim().parse().ok()?, dy.trim().parse().ok()?),
        None => (0, 0),
    };
    Some(Frame { x: x.trim().parse().ok()?, y: y.trim().parse().ok()?, keys, buttons, wheel, look, t })
}

pub fn parse(text: &str) -> Recording {
    let mut recording = Recording { poll_ms: OLD_POLL_MS, ..Recording::default() };
    for (number, line) in text.lines().enumerate() {
        let line = line.trim_start_matches('\u{FEFF}').trim();
        if line.is_empty() {
            continue;
        }
        if number == 0 && line.starts_with("WELFFI") {
            let header: Vec<&str> = line.split(' ').collect();
            if let Some(poll) = header.get(1).and_then(|text| text.parse().ok()) {
                recording.poll_ms = poll;
            }
            if let (Some(Ok(width)), Some(Ok(height))) = (header.get(2).map(|t| t.parse()), header.get(3).map(|t| t.parse())) {
                recording.width = width;
                recording.height = height;
            }
            continue;
        }
        if let Some(frame) = parse_frame(line, recording.frames.len(), recording.poll_ms) {
            recording.frames.push(frame);
        }
    }
    recording
}

pub fn to_text(recording: &Recording) -> String {
    let mut out = format!("WELFFI2 {} {} {}\n", recording.poll_ms, recording.width, recording.height);
    for frame in &recording.frames {
        out += &format!("{},{};{};{};{};{}", frame.x, frame.y, frame.keys.join(" "), frame.buttons.join(" "), frame.wheel, frame.t.round());
        if frame.look != (0, 0) {
            out += &format!(";{},{}", frame.look.0, frame.look.1);
        }
        out.push('\n');
    }
    out
}

fn is_real_input(frame: &Frame) -> bool {
    !frame.buttons.is_empty() || frame.wheel != 0 || frame.look != (0, 0) || frame.keys.iter().any(|key| !keys::is_modifier(key))
}

impl Recording {
    pub fn duration(&self) -> f64 {
        self.frames.last().map_or(0.0, |frame| frame.t)
    }

    fn tighten(&mut self) {
        let Some(last) = self.frames.iter().rposition(is_real_input) else {
            return;
        };
        let cut = self.frames[last].t + TAIL_PAD_MS;
        self.frames.retain(|frame| frame.t <= cut);
    }
}

pub fn load(path: &Path) -> Option<Recording> {
    let mut recording = parse(&std::fs::read_to_string(path).ok()?);
    if recording.width == 0 {
        recording.tighten();
    }
    Some(recording)
}

pub fn clock_text(ms: f64) -> String {
    let ms = ms.max(0.0) as u64;
    format!("{}:{:02}.{}", ms / 60_000, ms % 60_000 / 1000, ms % 1000 / 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_ahk_format() {
        let recording = parse("\u{FEFF}WELFFI2 4 800 600\r\n424,382;;;0;9\r\n395,424;i Right;Left;-2;353\r\n\r\n");
        assert_eq!((recording.poll_ms, recording.width, recording.height), (4, 800, 600));
        assert_eq!(recording.frames.len(), 2);
        let frame = &recording.frames[1];
        assert_eq!((frame.x, frame.y, frame.wheel, frame.t), (395, 424, -2, 353.0));
        assert_eq!((frame.keys.clone(), frame.buttons.clone()), (vec!["i".to_string(), "Right".to_string()], vec!["Left".to_string()]));
    }

    #[test]
    fn old_files_get_times_from_the_poll_rate() {
        let recording = parse("WELFFI 10\n1,2\n3,4;w\nbad line\n5,6;;Left\n");
        assert_eq!((recording.width, recording.poll_ms), (0, 10));
        assert_eq!(recording.frames.iter().map(|f| f.t).collect::<Vec<_>>(), [0.0, 10.0, 20.0]);
    }

    #[test]
    fn saving_and_reading_gives_the_same_frames() {
        let recording = parse("WELFFI2 8 1280 720\n1,2;w LShift;;0;0\n3,4;;Right;1;8\n400,300;;;0;16;-5,12\n");
        assert_eq!(recording.frames[2].look, (-5, 12));
        assert_eq!(recording.frames[1].look, (0, 0));
        assert_eq!(parse(&to_text(&recording)), recording);
        assert!(to_text(&recording).starts_with("WELFFI2 8 1280 720\n1,2;w LShift;;0;0\n"));
        assert!(to_text(&recording).ends_with("400,300;;;0;16;-5,12\n"));
    }

    #[test]
    fn tighten_cuts_idle_tail_but_not_modifiers_alone() {
        let mut lines = String::from("WELFFI 100\n0,0;;Left\n");
        for _ in 0..20 {
            lines += "0,0;LShift\n";
        }
        let mut recording = parse(&lines);
        recording.tighten();
        assert_eq!(recording.duration(), 600.0);
        let mut idle = parse("WELFFI 100\n0,0;LShift\n0,0\n");
        idle.tighten();
        assert_eq!(idle.frames.len(), 2);
    }

    #[test]
    fn clock_reads_minutes_seconds_tenths() {
        assert_eq!(clock_text(0.0), "0:00.0");
        assert_eq!(clock_text(65_432.0), "1:05.4");
    }
}
