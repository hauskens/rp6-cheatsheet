//! A character-cell drawing of the P-6 top panel, after the figure on p. 4 of
//! the Owner's Manual. Positions are approximate but the arrangement matches.

use std::collections::HashSet;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

/// Drawing size in cells (excluding the outer frame).
pub const W: u16 = 97;
pub const H: u16 = 16;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Button,
    Knob,
    Pad,
    Key,
    Jack,
    Display,
}

pub struct Widget {
    pub x: u16,
    pub y: u16,
    pub lines: &'static [&'static str],
    /// First id is the control this widget selects; the rest are groups it belongs to.
    pub ids: &'static [&'static str],
    pub kind: Kind,
}

impl Widget {
    pub fn width(&self) -> u16 {
        self.lines.iter().map(|l| l.chars().count() as u16).max().unwrap_or(0)
    }
    fn center(&self) -> (i32, i32) {
        (
            self.x as i32 + self.width() as i32 / 2,
            // Terminal cells are ~twice as tall as wide; weigh rows accordingly.
            (self.y as i32 + self.lines.len() as i32 / 2) * 2,
        )
    }
}

const fn w(x: u16, y: u16, lines: &'static [&'static str], ids: &'static [&'static str], kind: Kind) -> Widget {
    Widget { x, y, lines, ids, kind }
}

use Kind::*;

const fn key(i: u16, label: &'static [&'static str], id: &'static [&'static str]) -> Widget {
    w(1 + 6 * i, 12, label, id, Key)
}

pub static WIDGETS: &[Widget] = &[
    // top strip: jacks
    w(36, 0, &[" * CHARGE "], &["charge"], Jack),
    w(47, 0, &[" SYNC IN "], &["sync-in"], Jack),
    w(57, 0, &[" SYNC OUT "], &["sync-out"], Jack),
    w(68, 0, &[" MIX IN "], &["mix-in"], Jack),
    w(77, 0, &[" MIX OUT "], &["mix-out"], Jack),
    w(88, 0, &[" VOLUME "], &["volume"], Knob),
    // common
    w(2, 2, &[" 8.8.8.8. "], &["display"], Display),
    w(1, 4, &["    (o)    ", "TEMPO/VALUE"], &["tempo"], Knob),
    w(2, 6, &[" SHIFT "], &["shift"], Button),
    w(11, 6, &[" PATTERN "], &["pattern"], Button),
    w(2, 8, &["   >   "], &["play"], Button),
    w(11, 8, &["    O    "], &["rec"], Button),
    // effects
    w(24, 2, &["  (o)  ", " CTRL1 "], &["ctrl1"], Knob),
    w(33, 2, &["  (o)  ", " CTRL2 "], &["ctrl2"], Knob),
    w(42, 2, &["  (o)  ", " CTRL3 "], &["ctrl3"], Knob),
    w(23, 4, &[" LOOPER "], &["looper"], Button),
    w(32, 4, &[" PITCH "], &["pitch-fx"], Button),
    w(41, 4, &[" DELAY "], &["delay"], Button),
    w(23, 6, &[" FILTER "], &["filter"], Button),
    w(32, 6, &[" SCATTER "], &["scatter"], Button),
    w(42, 6, &[" MFX "], &["mfx"], Button),
    // sample editing
    w(53, 2, &["  (o)  ", " PITCH "], &["pitch"], Knob),
    w(63, 2, &["  (o)  ", " START "], &["start"], Knob),
    w(73, 2, &["  (o)  ", "  END  "], &["end"], Knob),
    w(83, 2, &["  (o)  ", " LEVEL "], &["level"], Knob),
    w(61, 4, &[" LO-FI "], &["lofi"], Button),
    w(70, 4, &[" GATE "], &["gate"], Button),
    w(52, 6, &[" SAMPLING "], &["sampling"], Button),
    w(63, 6, &[" DELETE "], &["delete"], Button),
    w(72, 6, &[" LOOP "], &["loop"], Button),
    // banks + granular + kybd
    w(81, 4, &[" A/E "], &["bank"], Button),
    w(88, 4, &[" B/F "], &["bank"], Button),
    w(81, 6, &[" C/G "], &["bank"], Button),
    w(88, 6, &[" D/H "], &["bank"], Button),
    w(86, 8, &[" GRANULAR "], &["granular"], Pad),
    w(90, 10, &[" KYBD "], &["kybd"], Button),
    // pads
    w(23, 8, &["       ", "   1   "], &["pads"], Pad),
    w(31, 8, &["       ", "   2   "], &["pads"], Pad),
    w(39, 8, &["       ", "   3   "], &["pads"], Pad),
    w(47, 8, &["       ", "   4   "], &["pads"], Pad),
    w(55, 8, &["       ", "   5   "], &["pads"], Pad),
    w(63, 8, &["       ", "   6   "], &["pads"], Pad),
    // step / keyboard row
    key(0, &["  C  "], &["k-c", "steps"]),
    key(1, &[" C#  "], &["k-cs", "steps"]),
    key(2, &["  D  "], &["k-d", "steps"]),
    key(3, &[" D#  "], &["k-ds", "steps"]),
    key(4, &["  E  "], &["k-e", "steps"]),
    key(5, &["  F  "], &["k-f", "steps"]),
    key(6, &[" F#  "], &["k-fs", "steps"]),
    key(7, &["  G  "], &["k-g", "steps"]),
    key(8, &[" G#  "], &["k-gs", "steps"]),
    key(9, &["  A  "], &["k-a", "steps"]),
    key(10, &[" A#  "], &["k-as", "steps"]),
    key(11, &["  B  "], &["k-b", "steps"]),
    key(12, &["  C  "], &["k-c2", "steps"]),
    key(13, &["OCT- "], &["oct-down", "steps"]),
    key(14, &["OCT+ "], &["oct-up", "steps"]),
    key(15, &[" TIE "], &["tie", "steps"]),
    // rear
    w(7, 15, &[" POWER "], &["power"], Jack),
    w(15, 15, &[" USB-C "], &["usb"], Jack),
    w(23, 15, &[" MIDI IN/OUT "], &["midi"], Jack),
];

/// Printed panel text that isn't a control.
static DECOR: &[(u16, u16, &str)] = &[
    (1, 0, "Roland  P-6"),
    (11, 7, "STEP LOOP"),
    (3, 9, "COPY"),
    (11, 9, "QUANTIZE"),
    (23, 7, "─────── EFFECTS ───────"),
    (71, 5, "POLY"),
    (53, 4, "MIC ="),
    (63, 7, "MIC MUTE"),
    (72, 7, "REVERSE"),
    (81, 8, "BANK"),
    (91, 11, "HOLD"),
    (37, 14, "─── SAMPLE EDIT ───────"),
    (67, 14, "─── PTN CLEAR ───"),
    (1, 15, "REAR"),
];

/// Function labels under each step key (from the panel print and the manual).
static KEY_LABELS: [&str; 16] = [
    "EXIT", "ENTER", "SHFL", "LAST", "<PAGE", "PAGE>", "P.ENV", "VOICE", "FILTR", "MIXER", "D/REV", "", "",
    "MOTN", "MENU", "WRITE",
];

pub struct Highlight<'a> {
    /// The selected entry's own control.
    pub primary: Option<&'a str>,
    /// Everything the entry involves (shown dimly when not part of the current frame).
    pub involved: HashSet<&'a str>,
    /// Current animation frame: keys held down, and the key being pressed now.
    pub held: &'a [&'a str],
    pub active: &'a [&'a str],
    /// Blink phase for `active`.
    pub blink_on: bool,
    /// Text for the LED display; None shows all segments dimly.
    pub display: Option<&'a str>,
}

pub const PURPLE: Color = Color::Rgb(185, 90, 255);
pub const PURPLE_DIM: Color = Color::Rgb(85, 45, 120);

fn lit() -> Style {
    Style::new().fg(Color::Black).bg(PURPLE).add_modifier(Modifier::BOLD)
}

/// The unit's body colour. Every cell of the drawing gets an explicit background, so
/// nothing depends on the terminal's default background.
const BODY: Color = Color::Rgb(28, 28, 32);

fn base_style(kind: Kind) -> Style {
    match kind {
        Button => Style::new().fg(Color::Rgb(230, 230, 230)).bg(Color::Rgb(60, 60, 66)),
        Knob => Style::new().fg(Color::Rgb(200, 200, 200)).bg(BODY),
        Pad => Style::new().fg(Color::Rgb(40, 40, 40)).bg(Color::Rgb(185, 185, 185)),
        Key => Style::new().fg(Color::Rgb(30, 30, 30)).bg(Color::Rgb(215, 215, 215)),
        Jack => Style::new().fg(Color::Rgb(150, 150, 150)).bg(Color::Rgb(35, 35, 38)),
        Display => Style::new().fg(Color::Rgb(255, 70, 50)).bg(Color::Rgb(25, 0, 0)),
    }
}

/// Render the unit into `area`'s top-left (caller provides at least W x H, extra is ignored).
pub fn render(buf: &mut Buffer, area: Rect, hl: &Highlight) {
    let put = |buf: &mut Buffer, x: u16, y: u16, s: &str, style: Style| {
        if y >= area.height || x >= area.width {
            return;
        }
        buf.set_stringn(area.x + x, area.y + y, s, (area.width - x) as usize, style);
    };

    let body = " ".repeat(W as usize);
    for y in 0..H {
        put(buf, 0, y, &body, Style::new().bg(BODY));
    }
    let decor = Style::new().fg(Color::Rgb(110, 110, 110)).bg(BODY);
    for &(x, y, s) in DECOR {
        put(buf, x, y, s, decor);
    }
    for (i, label) in KEY_LABELS.iter().enumerate() {
        put(buf, 1 + 6 * i as u16, 13, &format!("{label:^5}"), decor);
    }
    for wd in WIDGETS {
        let any = |set: &[&str]| wd.ids.iter().any(|id| set.contains(id));
        let style = if any(hl.active) {
            if hl.blink_on { lit() } else { base_style(wd.kind) }
        } else if any(hl.held) || hl.primary.is_some_and(|p| wd.ids.contains(&p)) {
            lit()
        } else if wd.ids.iter().any(|id| hl.involved.contains(id)) {
            Style::new().fg(Color::White).bg(PURPLE_DIM)
        } else {
            base_style(wd.kind)
        };
        let width = wd.width() as usize;
        if wd.kind == Display {
            let (text, led) = match hl.display {
                Some(code) => (code, Style::new().fg(Color::Rgb(255, 60, 40)).add_modifier(Modifier::BOLD)),
                None => ("8.8.8.8.", Style::new().fg(Color::Rgb(90, 25, 20))),
            };
            let text: String = format!("{text:^width$}").chars().take(width).collect();
            put(buf, wd.x, wd.y, &text, led.bg(Color::Rgb(25, 0, 0)));
            continue;
        }
        for (i, line) in wd.lines.iter().enumerate() {
            put(buf, wd.x, wd.y + i as u16, &format!("{line:^width$}"), style);
        }
    }
}

#[derive(Clone, Copy)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

/// Nearest widget from `from` in direction `dir`, preferring ones roughly in line.
pub fn neighbor(from: usize, dir: Dir) -> usize {
    let (fx, fy) = WIDGETS[from].center();
    let best = |cone: bool| {
        WIDGETS
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != from)
            .filter_map(|(i, wd)| {
                let (x, y) = wd.center();
                let (along, across) = match dir {
                    Dir::Right => (x - fx, y - fy),
                    Dir::Left => (fx - x, y - fy),
                    Dir::Down => (y - fy, x - fx),
                    Dir::Up => (fy - y, x - fx),
                };
                // Prefer targets within a 2:1 cone; fall back to the whole half-plane.
                let ok = along > 0 && (!cone || 2 * along > across.abs());
                ok.then_some((along + 3 * across.abs(), i))
            })
            .min()
            .map(|(_, i)| i)
    };
    best(true).or_else(|| best(false)).unwrap_or(from)
}

/// Widget for control `id`; group ids like "steps" resolve to their first member.
pub fn widget_for(id: &str) -> Option<usize> {
    WIDGETS.iter().position(|w| w.ids[0] == id).or_else(|| WIDGETS.iter().position(|w| w.ids.contains(&id)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Catalog;

    #[test]
    fn layout_fits_and_ids_exist() {
        let cat = Catalog::load().unwrap();
        let mut cells = HashSet::new();
        for wd in WIDGETS {
            assert!(wd.x + wd.width() <= W, "{:?} too wide", wd.ids);
            assert!(wd.y + wd.lines.len() as u16 <= H, "{:?} too tall", wd.ids);
            for id in wd.ids {
                assert!(cat.ids.contains_key(*id), "unknown control {id}");
            }
            for dy in 0..wd.lines.len() as u16 {
                for dx in 0..wd.width() {
                    assert!(cells.insert((wd.x + dx, wd.y + dy)), "{:?} overlaps", wd.ids);
                }
            }
        }
        // Every control in the panel chip rows is drawn somewhere.
        for (_, items) in &cat.panel {
            for &e in items {
                let id = cat.entries[e].id.as_deref().unwrap();
                assert!(widget_for(id).is_some(), "{id} not drawn");
            }
        }
    }

    #[test]
    fn navigation_moves_somewhere_sensible() {
        let shift = widget_for("shift").unwrap();
        assert_eq!(WIDGETS[neighbor(shift, Dir::Right)].ids[0], "pattern");
        assert_eq!(WIDGETS[neighbor(widget_for("k-c").unwrap(), Dir::Right)].ids[0], "k-cs");
    }
}
