//! Turn a human-written combo like "SHIFT + A# (DELAY/REVERB) → TEMPO/VALUE → C# (ENTER)"
//! into an ordered sequence of key presses for the animated unit drawing.
//!
//! Grammar (informal): stages are separated by `→` (or `…`); within a stage, parts are
//! separated by ` + ` or `,` and are held down together, in order. A part may list
//! alternatives with `/` ("E / F"), which light up together. Parenthesised text and
//! filler words ("Hold", "turn", "knob") are ignored, as are parts that name no control
//! (display codes like "Ptn", "choose destination").

/// One animation frame: `held` stay lit, `active` is the press happening now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub held: Vec<&'static str>,
    pub active: Vec<&'static str>,
}

const FILLER: &[&str] =
    &["hold", "turn", "press", "knob", "button", "buttons", "together", "pick", "choose", "dest", "on", "the"];

/// Panel label (lowercased) → control id.
const ALIASES: &[(&str, &str)] = &[
    ("shift", "shift"),
    ("pattern", "pattern"),
    ("kybd", "kybd"),
    ("mfx", "mfx"),
    ("▶", "play"),
    ("●", "rec"),
    ("tie", "tie"),
    ("oct+", "oct-up"),
    ("oct-", "oct-down"),
    ("c", "k-c"),
    ("exit", "k-c"),
    ("c#", "k-cs"),
    ("enter", "k-cs"),
    ("d", "k-d"),
    ("d#", "k-ds"),
    ("e", "k-e"),
    ("f", "k-f"),
    ("f#", "k-fs"),
    ("g", "k-g"),
    ("g#", "k-gs"),
    ("a", "k-a"),
    ("a#", "k-as"),
    ("lo-fi", "lofi"),
    ("gate", "gate"),
    ("loop", "loop"),
    ("delete", "delete"),
    ("sampling", "sampling"),
    ("granular", "granular"),
    ("pad", "pads"),
    ("pads", "pads"),
    ("pad 1–6", "pads"),
    ("bank", "bank"),
    ("bank a/e–d/h", "bank"),
    ("delay", "delay"),
    ("looper", "looper"),
    ("filter", "filter"),
    ("scatter", "scatter"),
    ("pitch", "pitch"),
    ("start", "start"),
    ("end", "end"),
    ("level", "level"),
    ("tempo/value", "tempo"),
    ("ctrl1", "ctrl1"),
    ("ctrl2", "ctrl2"),
    ("ctrl3", "ctrl3"),
    ("step", "steps"),
    ("step 1–4", "steps"),
    ("power", "power"),
];

fn alias(s: &str) -> Option<&'static str> {
    ALIASES.iter().find(|(k, _)| *k == s).map(|(_, v)| *v)
}

fn strip_parens(s: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for ch in s.chars() {
        match ch {
            '(' => depth += 1,
            ')' => depth = (depth - 1).max(0),
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    out
}

/// Control ids named by one `+`-part, or empty if it names none.
fn part_ids(part: &str) -> Vec<&'static str> {
    let words: Vec<&str> = part.split_whitespace().filter(|w| !FILLER.contains(w)).collect();
    let token = words.join(" ");
    if token.is_empty() {
        return Vec::new();
    }
    if let Some(id) = alias(&token) {
        return vec![id];
    }
    // Alternatives: "e / f", "pitch/start/end/level".
    let alts: Vec<&'static str> = token.split('/').filter_map(|t| alias(t.trim())).collect();
    if alts.len() > 1 { alts } else { Vec::new() }
}

/// Stages of parts; each part is a set of control ids pressed at the same moment.
pub fn parse(combo: &str) -> Vec<Vec<Vec<&'static str>>> {
    let text = strip_parens(&combo.to_lowercase()).replace('…', "→");
    text.split('→')
        .map(|stage| {
            stage
                .split(',')
                .flat_map(|s| s.split(" + "))
                .map(part_ids)
                .filter(|ids| !ids.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|stage| !stage.is_empty())
        .collect()
}

/// Expand stages into frames: within a stage keys accumulate (held + newly pressed),
/// and everything is released between stages.
pub fn frames(stages: &[Vec<Vec<&'static str>>]) -> Vec<Frame> {
    let mut out = Vec::new();
    for stage in stages {
        let mut held = Vec::new();
        for part in stage {
            out.push(Frame { held: held.clone(), active: part.clone() });
            held.extend(part.iter().copied());
        }
    }
    out
}

/// Every control id the parser can produce, for validation.
#[cfg(test)]
pub fn alias_ids() -> impl Iterator<Item = &'static str> {
    ALIASES.iter().map(|(_, v)| *v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(combo: &str) -> Vec<Vec<Vec<&'static str>>> {
        parse(combo)
    }

    #[test]
    fn simple_shift_combo() {
        let f = frames(&ids("SHIFT + A# (DELAY/REVERB)"));
        assert_eq!(
            f,
            vec![
                Frame { held: vec![], active: vec!["shift"] },
                Frame { held: vec!["shift"], active: vec!["k-as"] },
            ]
        );
    }

    #[test]
    fn stages_skip_unknown_tokens() {
        assert_eq!(
            ids("SHIFT + ▶ → Ptn → ENTER → choose destination → ENTER"),
            vec![vec![vec!["shift"], vec!["play"]], vec![vec!["k-cs"]], vec![vec!["k-cs"]]]
        );
    }

    #[test]
    fn oct_plus_is_not_split_and_parens_dropped() {
        assert_eq!(ids("KYBD + OCT+ (● off)"), vec![vec![vec!["kybd"], vec!["oct-up"]]]);
        assert_eq!(ids("MFX (+ DELAY) + GATE"), vec![vec![vec!["mfx"], vec!["gate"]]]);
    }

    #[test]
    fn alternatives_and_filler() {
        assert_eq!(ids("SHIFT + E / F"), vec![vec![vec!["shift"], vec!["k-e", "k-f"]]]);
        assert_eq!(
            ids("Hold step + turn PITCH/START/END/LEVEL"),
            vec![vec![vec!["steps"], vec!["pitch", "start", "end", "level"]]]
        );
        assert_eq!(ids("Hold ● + power on"), vec![vec![vec!["rec"], vec!["power"]]]);
        assert_eq!(ids("Hold step + GRANULAR, turn TEMPO/VALUE"), vec![vec![vec!["steps"], vec!["granular"], vec!["tempo"]]]);
    }
}
