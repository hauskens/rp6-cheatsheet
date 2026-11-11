use std::collections::{HashMap, HashSet};

use anyhow::{Result, bail};
use serde::Deserialize;

use crate::combo::{self, Frame};

const RAW: &str = include_str!("../data/p6.toml");

#[derive(Debug, Deserialize)]
struct Raw {
    group: Vec<Group>,
    section: Vec<Section>,
    entry: Vec<Entry>,
    panel: Vec<PanelRow>,
}

#[derive(Debug, Deserialize)]
pub struct Group {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct Section {
    pub id: String,
    pub group: String,
    pub title: String,
    /// Controls implied by every entry in the section (e.g. the menu's entry combo).
    #[serde(default)]
    pub controls: Vec<String>,
    /// How to reach this section's screen; played before an entry's own combo.
    pub combo: Option<String>,
    /// How to reach a parameter inside the screen, for entries without their own combo.
    pub nav: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Entry {
    pub section: String,
    pub name: String,
    pub id: Option<String>,
    pub short: Option<String>,
    pub display: Option<String>,
    pub combo: Option<String>,
    #[serde(default)]
    pub controls: Vec<String>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub values: Vec<(String, String)>,
    #[serde(default)]
    pub steps: Vec<String>,
    #[serde(default)]
    pub notes: Vec<String>,
    pub page: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct PanelRow {
    pub title: String,
    pub items: Vec<String>,
}

/// Indexed, validated view of the cheatsheet data.
pub struct Catalog {
    pub groups: Vec<Group>,
    pub sections: Vec<Section>,
    pub entries: Vec<Entry>,
    /// Panel rows, items resolved to entry indices.
    pub panel: Vec<(String, Vec<usize>)>,
    /// Section indices in display order (grouped).
    pub section_order: Vec<usize>,
    /// Entry indices per section index.
    pub by_section: Vec<Vec<usize>>,
    /// For control entries: other entries that reference the control.
    pub related: Vec<Vec<usize>>,
    /// Lowercased full text per entry, for search.
    pub haystack: Vec<String>,
    /// Control id → entry index.
    pub ids: HashMap<String, usize>,
    /// Animated key-press sequence per entry (empty if it has no combo).
    pub sequences: Vec<Vec<Frame>>,
}

impl Catalog {
    pub fn load() -> Result<Self> {
        let raw: Raw = toml::from_str(RAW)?;
        Self::build(raw)
    }

    fn build(raw: Raw) -> Result<Self> {
        let Raw { group: groups, section: sections, entry: entries, panel } = raw;

        let group_ids: HashSet<&str> = groups.iter().map(|g| g.id.as_str()).collect();
        let mut section_idx = HashMap::new();
        for (i, s) in sections.iter().enumerate() {
            if !group_ids.contains(s.group.as_str()) {
                bail!("section '{}' has unknown group '{}'", s.id, s.group);
            }
            if section_idx.insert(s.id.as_str(), i).is_some() {
                bail!("duplicate section id '{}'", s.id);
            }
        }

        let mut id_idx = HashMap::new();
        let mut by_section = vec![Vec::new(); sections.len()];
        for (i, e) in entries.iter().enumerate() {
            let Some(&s) = section_idx.get(e.section.as_str()) else {
                bail!("entry '{}' has unknown section '{}'", e.name, e.section);
            };
            by_section[s].push(i);
            if let Some(id) = &e.id
                && id_idx.insert(id.as_str(), i).is_some()
            {
                bail!("duplicate entry id '{id}'");
            }
        }

        for s in &sections {
            if let Some(c) = s.controls.iter().find(|c| !id_idx.contains_key(c.as_str())) {
                bail!("section '{}' references unknown control '{c}'", s.id);
            }
        }

        let mut related = vec![Vec::new(); entries.len()];
        for (i, e) in entries.iter().enumerate() {
            for c in &e.controls {
                let Some(&target) = id_idx.get(c.as_str()) else {
                    bail!("entry '{}' references unknown control '{c}'", e.name);
                };
                if target != i && !related[target].contains(&i) {
                    related[target].push(i);
                }
            }
        }

        let mut panel_rows = Vec::new();
        for row in panel {
            let mut items = Vec::new();
            for id in &row.items {
                let Some(&i) = id_idx.get(id.as_str()) else {
                    bail!("panel row '{}' references unknown control '{id}'", row.title);
                };
                items.push(i);
            }
            panel_rows.push((row.title, items));
        }

        let section_order = groups
            .iter()
            .flat_map(|g| {
                sections
                    .iter()
                    .enumerate()
                    .filter(move |(_, s)| s.group == g.id)
                    .map(|(i, _)| i)
            })
            .collect();

        let haystack = entries.iter().map(full_text).collect();
        let ids = id_idx.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
        let sequences = entries
            .iter()
            .map(|e| sequence(&sections[section_idx[e.section.as_str()]], e))
            .collect();

        Ok(Self {
            ids,
            sequences,
            groups,
            sections,
            entries,
            panel: panel_rows,
            section_order,
            by_section,
            related,
            haystack,
        })
    }

    pub fn section_of(&self, entry: usize) -> usize {
        let name = &self.entries[entry].section;
        self.sections.iter().position(|s| &s.id == name).expect("validated")
    }

    /// Controls to light up for an entry: (its own control id, everything it involves).
    pub fn highlights(&self, entry: usize) -> (Option<&str>, HashSet<&str>) {
        let e = &self.entries[entry];
        let sec = &self.sections[self.section_of(entry)];
        let involved = e.controls.iter().chain(&sec.controls).map(String::as_str).collect();
        (e.id.as_deref(), involved)
    }

    /// The entry's display code(s), as shown on the 4-digit LED.
    pub fn display_codes(&self, entry: usize) -> Vec<&str> {
        self.entries[entry].display.as_deref().map_or_else(Vec::new, |d| d.split(" / ").collect())
    }

    pub fn group_title(&self, section: usize) -> &str {
        let gid = &self.sections[section].group;
        self.groups.iter().find(|g| &g.id == gid).map_or("", |g| &g.title)
    }
}

/// Section combo (to reach the screen) followed by the entry's combo, or the section's
/// generic `nav` if the entry has none. The section part is skipped when the entry's
/// combo already includes all of its controls (e.g. "SHIFT + ▶ → Ptn" in COPY).
fn sequence(sec: &Section, e: &Entry) -> Vec<Frame> {
    let sec_stages = sec.combo.as_deref().map(combo::parse).unwrap_or_default();
    let own = e.combo.as_deref().or(if sec_stages.is_empty() { None } else { sec.nav.as_deref() });
    let own_stages = own.map(combo::parse).unwrap_or_default();
    let ids = |stages: &[Vec<Vec<&'static str>>]| -> HashSet<&'static str> {
        stages.iter().flatten().flatten().copied().collect()
    };
    let stages = if !sec_stages.is_empty() && !ids(&sec_stages).is_subset(&ids(&own_stages)) {
        sec_stages.into_iter().chain(own_stages).collect()
    } else {
        own_stages
    };
    combo::frames(&stages)
}

fn full_text(e: &Entry) -> String {
    let mut s = String::new();
    for part in [&e.name, &e.summary]
        .into_iter()
        .chain(e.short.iter())
        .chain(e.display.iter())
        .chain(e.combo.iter())
        .chain(e.steps.iter())
        .chain(e.notes.iter())
    {
        s.push_str(part);
        s.push('\n');
    }
    for (v, d) in &e.values {
        s.push_str(v);
        s.push(' ');
        s.push_str(d);
        s.push('\n');
    }
    s.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_data_is_valid() {
        let cat = Catalog::load().unwrap();
        assert!(cat.entries.len() > 100);
        assert_eq!(cat.section_order.len(), cat.sections.len());
        assert!(cat.by_section.iter().all(|v| !v.is_empty()), "empty section");
    }

    #[test]
    fn display_codes_fit_four_digits() {
        let cat = Catalog::load().unwrap();
        for i in 0..cat.entries.len() {
            for code in cat.display_codes(i) {
                // Dots light up beside a digit; they don't take a digit of their own.
                let digits = code.chars().filter(|&c| c != '.').count();
                assert!(digits <= 4, "'{code}' ({}) needs {digits} digits", cat.entries[i].name);
            }
        }
    }

    #[test]
    fn combo_aliases_point_at_real_controls() {
        let cat = Catalog::load().unwrap();
        for id in combo::alias_ids() {
            assert!(cat.ids.contains_key(id), "alias target '{id}' missing");
        }
    }

    fn seq(cat: &Catalog, name: &str) -> Vec<Vec<&'static str>> {
        let i = cat.entries.iter().position(|e| e.name == name).unwrap();
        cat.sequences[i].iter().map(|f| f.active.clone()).collect()
    }

    #[test]
    fn sequences_include_section_entry_combo() {
        let cat = Catalog::load().unwrap();
        // Knob shortcut inside a SAMPLE EDIT screen: open the screen, then turn the knob.
        assert_eq!(seq(&cat, "P.Env Attack"), vec![vec!["shift"], vec!["k-fs"], vec!["pitch"]]);
        // Plain menu item: open MENU, pick with TEMPO/VALUE, ENTER.
        assert_eq!(seq(&cat, "Volume"), vec![vec!["shift"], vec!["oct-up"], vec!["tempo"], vec!["k-cs"]]);
        // Entry combo already starts with the section's combo: no duplication.
        assert_eq!(seq(&cat, "Copy pattern"), vec![vec!["shift"], vec!["play"], vec!["k-cs"], vec!["k-cs"]]);
        assert_eq!(seq(&cat, "Delay/reverb settings"), vec![vec!["shift"], vec!["k-as"]]);
    }
}
