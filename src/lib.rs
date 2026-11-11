//! Roland P-6 cheatsheet: the app state and drawing, shared by the terminal and web
//! front ends (`src/main.rs`).

mod combo;
mod data;
mod search;
mod ui;
mod unit;

use anyhow::Result;
use fuzzy_matcher::skim::SkimMatcherV2;
use ratatui::widgets::ListState;
use web_time::Instant;

use data::Catalog;

pub use ui::draw;

/// Running in the browser: there is no quitting or repainting the terminal.
pub const WEB: bool = cfg!(target_arch = "wasm32");

/// A key press, independent of the terminal (crossterm) or browser (Ratzilla) backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyCode {
    Char(char),
    Enter,
    Esc,
    Backspace,
    Tab,
    BackTab,
    Left,
    Right,
    Up,
    Down,
    Home,
    PageUp,
    PageDown,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    pub code: KeyCode,
    pub ctrl: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum View {
    Panel,
    Browse,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Sections,
    Entries,
    Detail,
}

pub struct Search {
    pub query: String,
    pub results: Vec<usize>,
    pub state: ListState,
}

pub struct App {
    pub cat: Catalog,
    matcher: SkimMatcherV2,
    pub view: View,
    pub focus: Focus,
    /// Index into `cat.section_order`.
    pub sec_pos: usize,
    pub sec_state: ListState,
    pub ent_state: ListState,
    pub panel_row: usize,
    pub panel_col: usize,
    /// Show the drawing of the unit (`v`). In Panel view it replaces the chip rows.
    pub show_unit: bool,
    /// Selected widget in the unit drawing (Panel view).
    pub unit_sel: usize,
    /// Entry shown on the unit, and when it (or the current step) was shown; drives
    /// blinking and display-code cycling.
    pub anim_entry: Option<usize>,
    pub anim_start: Instant,
    /// Key-press step being shown (Space/n, p); None = overview of the whole sequence.
    pub step: Option<usize>,
    pub detail_scroll: u16,
    pub search: Option<Search>,
    pub show_help: bool,
    /// Selected "Used in" link in the Detail pane (Browse view, Detail focused).
    pub link_sel: Option<usize>,
    /// Entries we jumped away from, for Backspace.
    history: Vec<usize>,
    /// Ctrl-L: clear the terminal and repaint everything on the next frame.
    pub needs_clear: bool,
    pub quit: bool,
}

impl App {
    /// Load the built-in catalog and start in the Panel view.
    pub fn load() -> Result<Self> {
        Ok(Self::new(Catalog::load()?))
    }

    fn new(cat: Catalog) -> Self {
        Self {
            cat,
            matcher: SkimMatcherV2::default(),
            view: View::Panel,
            focus: Focus::Sections,
            sec_pos: 0,
            sec_state: ListState::default(),
            ent_state: ListState::default().with_selected(Some(0)),
            panel_row: 0,
            panel_col: 0,
            show_unit: true,
            unit_sel: unit::widget_for("shift").unwrap_or(0),
            anim_entry: None,
            anim_start: Instant::now(),
            step: None,
            detail_scroll: 0,
            search: None,
            show_help: false,
            link_sel: None,
            history: Vec::new(),
            needs_clear: false,
            quit: false,
        }
    }

    pub fn section(&self) -> usize {
        self.cat.section_order[self.sec_pos]
    }

    pub fn section_entries(&self) -> &[usize] {
        &self.cat.by_section[self.section()]
    }

    /// The entry whose details are shown.
    pub fn current(&self) -> Option<usize> {
        if let Some(s) = &self.search {
            return s.state.selected().and_then(|i| s.results.get(i).copied());
        }
        match self.view {
            View::Panel if self.show_unit => self.cat.ids.get(unit::WIDGETS[self.unit_sel].ids[0]).copied(),
            View::Panel => self.cat.panel.get(self.panel_row).and_then(|(_, items)| items.get(self.panel_col).copied()),
            View::Browse => self.ent_state.selected().and_then(|i| self.section_entries().get(i).copied()),
        }
    }

    fn jump_to(&mut self, entry: usize) {
        let sec = self.cat.section_of(entry);
        self.sec_pos = self.cat.section_order.iter().position(|&s| s == sec).unwrap_or(0);
        let pos = self.cat.by_section[sec].iter().position(|&e| e == entry).unwrap_or(0);
        self.ent_state.select(Some(pos));
        self.view = View::Browse;
        self.focus = Focus::Entries;
        self.detail_scroll = 0;
        self.link_sel = None;
    }

    /// Jump to `entry`, remembering where we came from for Backspace.
    fn open(&mut self, entry: usize) {
        if let Some(cur) = self.current()
            && cur != entry
        {
            self.history.push(cur);
        }
        self.jump_to(entry);
    }

    fn back(&mut self) {
        if let Some(prev) = self.history.pop() {
            self.jump_to(prev);
        }
    }

    /// "Used in" entries of the current entry (only controls have them).
    pub fn links(&self) -> &[usize] {
        self.current().map_or(&[], |e| &self.cat.related[e])
    }

    /// Focus the Detail pane, selecting the first link if there is one.
    fn focus_detail(&mut self) {
        self.focus = Focus::Detail;
        self.link_sel = (!self.links().is_empty()).then_some(0);
    }

    pub fn handle(&mut self, key: Key) {
        if key.ctrl && key.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        if key.ctrl && key.code == KeyCode::Char('l') {
            self.needs_clear = true;
            return;
        }
        if self.show_help {
            self.show_help = false;
            return;
        }
        if self.search.is_some() {
            self.handle_search(key);
            return;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Char('?') => self.show_help = true,
            KeyCode::Backspace => self.back(),
            KeyCode::Char('n') => self.step_by(true),
            KeyCode::Char('p') => self.step_by(false),
            KeyCode::Char('v') => {
                // Keep the same control selected when switching panel styles.
                let cur = self.current();
                self.show_unit = !self.show_unit;
                if self.view == View::Panel
                    && let Some(id) = cur.and_then(|e| self.cat.entries[e].id.clone())
                {
                    self.select_control(&id);
                }
            }
            KeyCode::Char('/') => {
                self.search = Some(Search { query: String::new(), results: Vec::new(), state: ListState::default() });
                self.detail_scroll = 0;
            }
            KeyCode::Tab | KeyCode::BackTab => {
                self.view = match self.view {
                    View::Panel => View::Browse,
                    View::Browse => View::Panel,
                };
                self.detail_scroll = 0;
            }
            KeyCode::PageDown | KeyCode::Char('J') => self.detail_scroll = self.detail_scroll.saturating_add(5),
            KeyCode::PageUp | KeyCode::Char('K') => self.detail_scroll = self.detail_scroll.saturating_sub(5),
            _ => match self.view {
                View::Panel => self.handle_panel(key),
                View::Browse => self.handle_browse(key),
            },
        }
    }

    /// Number of key-press steps for the current entry.
    pub fn step_count(&self) -> usize {
        self.current().map_or(0, |e| self.cat.sequences[e].len())
    }

    /// Move through the key-press steps: overview → 1 → … → n → overview.
    fn step_by(&mut self, forward: bool) {
        let n = self.step_count();
        if n == 0 {
            return;
        }
        self.step = match (self.step, forward) {
            (None, true) => Some(0),
            (Some(i), true) if i + 1 < n => Some(i + 1),
            (None, false) => Some(n - 1),
            (Some(i), false) if i > 0 => Some(i - 1),
            _ => None,
        };
        // Restart the blink so the key to press shows immediately.
        self.anim_start = Instant::now();
    }

    fn select_control(&mut self, id: &str) {
        if let Some(w) = unit::widget_for(id) {
            self.unit_sel = w;
        }
        for (r, (_, items)) in self.cat.panel.iter().enumerate() {
            if let Some(c) = items.iter().position(|&e| self.cat.entries[e].id.as_deref() == Some(id)) {
                (self.panel_row, self.panel_col) = (r, c);
            }
        }
    }

    fn handle_panel(&mut self, key: Key) {
        if self.show_unit {
            let dir = match key.code {
                KeyCode::Left | KeyCode::Char('h') => Some(unit::Dir::Left),
                KeyCode::Right | KeyCode::Char('l') => Some(unit::Dir::Right),
                KeyCode::Up | KeyCode::Char('k') => Some(unit::Dir::Up),
                KeyCode::Down | KeyCode::Char('j') => Some(unit::Dir::Down),
                _ => None,
            };
            if let Some(d) = dir {
                self.unit_sel = unit::neighbor(self.unit_sel, d);
                self.detail_scroll = 0;
            } else if key.code == KeyCode::Enter
                && let Some(e) = self.current()
            {
                self.open(e);
                self.focus_detail();
            }
            return;
        }
        let rows = &self.cat.panel;
        let len = |r: usize| rows[r].1.len();
        let before = (self.panel_row, self.panel_col);
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => self.panel_col = self.panel_col.saturating_sub(1),
            KeyCode::Right | KeyCode::Char('l') => self.panel_col = (self.panel_col + 1).min(len(self.panel_row) - 1),
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                let up = matches!(key.code, KeyCode::Up | KeyCode::Char('k'));
                let new_row = if up {
                    self.panel_row.saturating_sub(1)
                } else {
                    (self.panel_row + 1).min(rows.len() - 1)
                };
                if new_row != self.panel_row {
                    // Keep roughly the same horizontal position across rows of different length.
                    let (old_len, new_len) = (len(self.panel_row), len(new_row));
                    self.panel_col = (self.panel_col * new_len / old_len.max(1)).min(new_len - 1);
                    self.panel_row = new_row;
                }
            }
            KeyCode::Enter => {
                if let Some(e) = self.current() {
                    self.jump_to(e);
                }
            }
            _ => {}
        }
        if before != (self.panel_row, self.panel_col) {
            self.detail_scroll = 0;
        }
    }

    fn handle_browse(&mut self, key: Key) {
        let up = matches!(key.code, KeyCode::Up | KeyCode::Char('k'));
        let down = matches!(key.code, KeyCode::Down | KeyCode::Char('j'));
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                self.focus = match self.focus {
                    Focus::Detail => Focus::Entries,
                    _ => Focus::Sections,
                };
                self.link_sel = None;
            }
            // In Detail, Enter opens the selected "Used in" link, or else steps
            // through the key presses (controls have links, combos have steps).
            KeyCode::Enter if self.focus == Focus::Detail => {
                if let Some(&target) = self.link_sel.and_then(|i| self.links().get(i)) {
                    self.open(target);
                    self.focus_detail();
                } else {
                    self.step_by(true);
                }
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => match self.focus {
                Focus::Sections => self.focus = Focus::Entries,
                Focus::Entries => self.focus_detail(),
                Focus::Detail => {}
            },
            KeyCode::Home | KeyCode::Char('g') if self.focus == Focus::Detail => self.detail_scroll = 0,
            _ if up || down => match self.focus {
                Focus::Sections => {
                    let n = self.cat.section_order.len();
                    self.sec_pos = if up { self.sec_pos.saturating_sub(1) } else { (self.sec_pos + 1).min(n - 1) };
                    self.ent_state.select(Some(0));
                    self.detail_scroll = 0;
                }
                Focus::Entries => {
                    let n = self.section_entries().len();
                    let cur = self.ent_state.selected().unwrap_or(0);
                    let next = if up { cur.saturating_sub(1) } else { (cur + 1).min(n - 1) };
                    self.ent_state.select(Some(next));
                    self.detail_scroll = 0;
                }
                // With links, j/k pick one (J/K still scroll); otherwise j/k scroll.
                Focus::Detail if !self.links().is_empty() => {
                    let n = self.links().len();
                    self.link_sel = Some(match self.link_sel {
                        None => 0,
                        Some(i) if up => i.saturating_sub(1),
                        Some(i) => (i + 1).min(n - 1),
                    });
                }
                Focus::Detail => {
                    self.detail_scroll =
                        if up { self.detail_scroll.saturating_sub(1) } else { self.detail_scroll.saturating_add(1) };
                }
            },
            _ => {}
        }
    }

    fn handle_search(&mut self, key: Key) {
        let Some(s) = self.search.as_mut() else { return };
        let ctrl = key.ctrl;
        let n = s.results.len();
        let cur = s.state.selected().unwrap_or(0);
        match key.code {
            KeyCode::Esc => self.search = None,
            KeyCode::Enter => {
                let target = s.state.selected().and_then(|i| s.results.get(i).copied());
                self.search = None;
                if let Some(e) = target {
                    self.open(e);
                }
            }
            KeyCode::Up | KeyCode::BackTab => s.state.select(Some(cur.saturating_sub(1))),
            KeyCode::Char('p') if ctrl => s.state.select(Some(cur.saturating_sub(1))),
            KeyCode::Down | KeyCode::Tab => s.state.select(Some((cur + 1).min(n.saturating_sub(1)))),
            KeyCode::Char('n') if ctrl => s.state.select(Some((cur + 1).min(n.saturating_sub(1)))),
            KeyCode::Char('u') if ctrl => self.set_query(String::new()),
            KeyCode::Backspace => {
                let mut q = s.query.clone();
                q.pop();
                self.set_query(q);
            }
            KeyCode::Char(c) if !ctrl => {
                let q = format!("{}{c}", s.query);
                self.set_query(q);
            }
            _ => {}
        }
        self.detail_scroll = 0;
    }

    fn set_query(&mut self, query: String) {
        let results = search::search(&self.cat, &self.matcher, &query);
        let sel = (!results.is_empty()).then_some(0);
        self.search = Some(Search { query, results, state: ListState::default().with_selected(sel) });
    }
}
