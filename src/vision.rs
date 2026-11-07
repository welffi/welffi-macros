use crate::capture::{self, Shot};
use crate::roblox::Rect;
use std::path::Path;

const MISS_SHARE: usize = 100;

pub struct Template {
    pub width: usize,
    pub height: usize,
    rgba: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Found {
    pub x: i32,
    pub y: i32,
}

pub fn load(path: &Path) -> Option<Template> {
    let image = image::open(path).ok()?.to_rgba8();
    Some(Template { width: image.width() as usize, height: image.height() as usize, rgba: image.into_raw() })
}

pub fn tolerance(base: i32, leniency: i32) -> u8 {
    (base + leniency - 50).clamp(5, 220) as u8
}

fn close(shot: &Shot, at: usize, template: &Template, pixel: usize, tolerance: u8) -> bool {
    let want = &template.rgba[pixel * 4..pixel * 4 + 4];
    if want[3] == 0 {
        return true;
    }
    let have = &shot.bgra[at * 4..at * 4 + 3];
    have[2].abs_diff(want[0]) <= tolerance && have[1].abs_diff(want[1]) <= tolerance && have[0].abs_diff(want[2]) <= tolerance
}

fn allowed_misses(template: &Template) -> usize {
    template.width * template.height / MISS_SHARE
}

fn matches_at(shot: &Shot, template: &Template, x: usize, y: usize, tolerance: u8) -> bool {
    let mut left = allowed_misses(template);
    for row in 0..template.height {
        for col in 0..template.width {
            if !close(shot, (y + row) * shot.width + x + col, template, row * template.width + col, tolerance) {
                if left == 0 {
                    return false;
                }
                left -= 1;
            }
        }
    }
    true
}

pub fn find_in(shot: &Shot, template: &Template, tolerance: u8) -> Option<(usize, usize)> {
    if template.width == 0 || template.width > shot.width || template.height > shot.height {
        return None;
    }
    for y in 0..=shot.height - template.height {
        for x in 0..=shot.width - template.width {
            if matches_at(shot, template, x, y, tolerance) {
                return Some((x, y));
            }
        }
    }
    None
}

pub fn find(area: Rect, templates: &[Template], tolerance: u8) -> Result<Option<Found>, String> {
    let shot = capture::grab(area).ok_or("couldn't read the screen")?;
    Ok(templates.iter().find_map(|template| find_in(&shot, template, tolerance)).map(|(x, y)| Found { x: area.x + x as i32, y: area.y + y as i32 }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shot(width: usize, height: usize, paint: impl Fn(usize, usize) -> [u8; 3]) -> Shot {
        let mut bgra = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let [r, g, b] = paint(x, y);
                bgra.extend([b, g, r, 255]);
            }
        }
        Shot { width, height, bgra }
    }

    fn template(width: usize, height: usize, pixels: &[[u8; 4]]) -> Template {
        Template { width, height, rgba: pixels.concat() }
    }

    #[test]
    fn finds_the_first_match_reading_rows() {
        let screen = shot(6, 4, |x, y| if (x, y) == (3, 1) || (x, y) == (1, 2) { [200, 10, 10] } else { [0, 0, 0] });
        let red = template(1, 1, &[[200, 10, 10, 255]]);
        assert_eq!(find_in(&screen, &red, 5), Some((3, 1)));
    }

    #[test]
    fn tolerance_is_per_channel() {
        let screen = shot(3, 3, |_, _| [100, 100, 100]);
        let near = template(2, 2, &[[110, 95, 100, 255]; 4]);
        assert_eq!(find_in(&screen, &near, 10), Some((0, 0)));
        assert_eq!(find_in(&screen, &near, 9), None);
    }

    #[test]
    fn transparent_pixels_match_anything() {
        let screen = shot(4, 1, |x, _| if x == 2 { [9, 9, 9] } else { [50, 60, 70] });
        let holed = template(2, 1, &[[0, 0, 0, 0], [50, 60, 70, 255]]);
        assert_eq!(find_in(&screen, &holed, 0), Some((0, 0)));
        let solid = template(2, 1, &[[9, 9, 9, 255], [50, 60, 70, 255]]);
        assert_eq!(find_in(&screen, &solid, 0), Some((2, 0)));
    }

    #[test]
    fn one_in_a_hundred_pixels_may_miss() {
        let screen = shot(100, 2, |x, y| if y == 0 && x == 7 { [255, 0, 0] } else { [0, 0, 0] });
        let black = template(100, 1, &[[0, 0, 0, 255]; 100]);
        assert_eq!(find_in(&screen, &black, 0), Some((0, 0)));
        let screen = shot(100, 1, |x, _| if x == 7 || x == 8 { [255, 0, 0] } else { [0, 0, 0] });
        assert_eq!(find_in(&screen, &black, 0), None);
    }

    #[test]
    fn template_bigger_than_screen_is_not_found() {
        let screen = shot(2, 2, |_, _| [0, 0, 0]);
        assert_eq!(find_in(&screen, &template(3, 1, &[[0, 0, 0, 255]; 3]), 255), None);
    }

    #[test]
    fn leniency_moves_tolerance_within_limits() {
        assert_eq!(tolerance(60, 50), 60);
        assert_eq!(tolerance(60, 100), 110);
        assert_eq!(tolerance(10, 0), 5);
        assert_eq!(tolerance(200, 100), 220);
    }
}
