use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap};

use ratatui::Frame;

use crate::combo::Frame as KeyFrame;
use crate::{App, Focus, View, unit};

const ACCENT: Color = Color::Rgb(255, 140, 40); // P-6 orange
const LED: Color = Color::Rgb(255, 70, 50);
const DIM: Color = Color::DarkGray;
const KEY: Color = Color::Yellow;
const VAL: Color = Color::Green;

pub fn draw(f: &mut Frame, app: &mut App) {
    // A new entry starts at the overview, with display codes cycling from the first.
    let cur = app.current();
    if cur != app.anim_entry {
        app.anim_entry = cur;
        app.anim_start = web_time::Instant::now();
        app.step = None;
    }

    let [top, body, bottom] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(5), Constraint::Length(1)]).areas(f.area());

    draw_tabs(f, app, top);
    match (app.view, app.search.is_some()) {
        (_, true) | (View::Browse, false) => draw_browse(f, app, body),
        (View::Panel, false) => draw_panel(f, app, body),
    }
    draw_footer(f, app, bottom);
    if app.show_help {
        draw_help(f);
    }
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let tab = |label: &str, on: bool| {
        if on {
            Span::styled(format!(" {label} "), Style::new().fg(Color::Black).bg(ACCENT).bold())
        } else {
            Span::styled(format!(" {label} "), Style::new().fg(DIM))
        }
    };
    let searching = app.search.is_some();
    let line = Line::from(vec![
        Span::styled(" P-6 ", Style::new().fg(ACCENT).bold()),
        Span::raw("cheatsheet  "),
        tab("Panel", app.view == View::Panel && !searching),
        tab("Browse", app.view == View::Browse && !searching),
        tab("Search", searching),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let hint = |k: &str, d: &str| vec![Span::styled(k.to_string(), Style::new().fg(KEY)), Span::raw(format!(" {d}  "))];
    let line = if let Some(s) = &app.search {
        Line::from(vec![
            Span::styled(" / ", Style::new().fg(Color::Black).bg(KEY)),
            Span::raw(format!(" {}", s.query)),
            Span::styled("█", Style::new().fg(KEY)),
            Span::styled("   ↑↓ select · Enter open · Esc cancel", Style::new().fg(DIM)),
        ])
    } else {
        let mut spans = vec![Span::raw(" ")];
        spans.extend(hint("Tab", "view"));
        spans.extend(hint("/", "search"));
        spans.extend(hint("v", if app.show_unit { "hide unit" } else { "show unit" }));
        spans.extend(hint("hjkl/←↓↑→", "move"));
        if app.view == View::Panel {
            spans.extend(hint("Enter", "open in Browse"));
        }
        spans.extend(hint("J/K", "scroll detail"));
        spans.extend(hint("?", "help"));
        if !crate::WEB {
            spans.extend(hint("q", "quit"));
        }
        Line::from(spans)
    };
    f.render_widget(Paragraph::new(line), area);
}

fn pane(title: &str, focused: bool) -> Block<'_> {
    let style = if focused { Style::new().fg(ACCENT) } else { Style::new().fg(DIM) };
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(style)
        .title(Span::styled(format!(" {title} "), style.bold()))
}

fn highlight() -> Style {
    Style::new().fg(Color::Black).bg(ACCENT).add_modifier(Modifier::BOLD)
}

// ───────────────────────────── panel view ─────────────────────────────

/// Height of the unit pane including its border.
const UNIT_H: u16 = unit::H + 2;

/// Draw the unit with `entry`'s controls lit. Centered horizontally.
fn draw_unit(f: &mut Frame, app: &App, area: Rect, entry: Option<usize>) {
    let w = (unit::W + 2).min(area.width);
    let rect = Rect { x: area.x + (area.width - w) / 2, width: w, height: UNIT_H.min(area.height), ..area };
    let (primary, involved) = entry.map(|e| app.cat.highlights(e)).unwrap_or_default();
    let frames: &[KeyFrame] = entry.map_or(&[], |e| &app.cat.sequences[e]);
    let elapsed = app.anim_start.elapsed().as_millis();
    let step = app.step.filter(|&i| i < frames.len());

    // Overview: every key in the sequence, steady. Step mode: that step only, with the
    // key to press blinking.
    let all_keys: Vec<&str> = frames.iter().flat_map(|fr| fr.active.iter().copied()).collect();
    let (held, active): (&[&str], &[&str]) = match step {
        Some(i) => (&frames[i].held, &frames[i].active),
        None => (&all_keys, &[]),
    };
    let blink_on = (elapsed / BLINK_MS).is_multiple_of(2);

    // Top border: the sequence spelled out, current step highlighted.
    let mut title = vec![Span::styled(" P-6 ", Style::new().fg(DIM).bold())];
    if !frames.is_empty() {
        title.push(Span::raw(" "));
        for (i, fr) in frames.iter().enumerate() {
            if i > 0 {
                let sep = if fr.held.is_empty() { " → " } else { " + " };
                title.push(Span::styled(sep, Style::new().fg(DIM)));
            }
            let label = fr.active.iter().map(|id| key_label(app, id)).collect::<Vec<_>>().join("/");
            let style = match step {
                Some(s) if s == i => Style::new().fg(Color::Black).bg(unit::PURPLE).bold(),
                Some(s) if i > s => Style::new().fg(DIM),
                _ => Style::new().fg(Color::White),
            };
            title.push(Span::styled(label, style));
        }
        title.push(Span::raw(" "));
    }

    let mut legend = Vec::new();
    if !frames.is_empty() {
        let hint = match step {
            Some(i) => format!(" step {}/{} · Enter/n next · p back ", i + 1, frames.len()),
            None => " Enter in Detail (or n): step through ".to_string(),
        };
        legend.push(Span::styled(hint, Style::new().fg(KEY)));
    }
    legend.extend([
        Span::styled("  ", Style::new().bg(unit::PURPLE)),
        Span::styled(" press ", Style::new().fg(DIM)),
        Span::styled("  ", Style::new().bg(unit::PURPLE_DIM)),
        Span::styled(" involved · v hide ", Style::new().fg(DIM)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(DIM))
        .title(Line::from(title))
        .title_bottom(Line::from(legend).right_aligned());
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    // Several codes (e.g. the four T.Env stages) take turns on the display.
    let codes = entry.map(|e| app.cat.display_codes(e)).unwrap_or_default();
    let display = (!codes.is_empty()).then(|| codes[(elapsed / DISPLAY_MS) as usize % codes.len()]);
    unit::render(f.buffer_mut(), inner, &unit::Highlight { primary, involved, held, active, blink_on, display });
}

/// Short panel label for a control id, e.g. "A#·DLY/REV".
fn key_label<'a>(app: &'a App, id: &'a str) -> &'a str {
    app.cat.ids.get(id).and_then(|&e| app.cat.entries[e].short.as_deref()).unwrap_or(id)
}

/// How long each display code shows when an entry has several.
const DISPLAY_MS: u128 = 1200;
/// Blink half-period for the key to press in step mode.
const BLINK_MS: u128 = 300;

fn draw_panel(f: &mut Frame, app: &mut App, area: Rect) {
    if app.show_unit {
        // Unit at the bottom, as in Browse view.
        let [top, bottom] = Layout::vertical([Constraint::Min(4), Constraint::Length(UNIT_H)]).areas(area);
        draw_detail(f, app, top, false);
        draw_unit(f, app, bottom, app.current());
        return;
    }
    let mut lines = Vec::new();
    for (r, (title, items)) in app.cat.panel.iter().enumerate() {
        lines.push(Line::styled(title.to_uppercase(), Style::new().fg(DIM).bold()));
        let mut spans = Vec::new();
        for (c, &e) in items.iter().enumerate() {
            let entry = &app.cat.entries[e];
            let label = entry.short.as_deref().unwrap_or(&entry.name);
            let style = if (r, c) == (app.panel_row, app.panel_col) {
                highlight()
            } else {
                Style::new().fg(Color::White).bg(Color::Rgb(45, 45, 50))
            };
            spans.push(Span::styled(format!(" {label} "), style));
            spans.push(Span::raw(" "));
        }
        lines.push(Line::from(spans));
        lines.push(Line::raw(""));
    }

    // Measure how tall the wrapped panel is so the detail gets the rest.
    let inner_w = area.width.saturating_sub(2).max(1);
    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    let needed = (para.line_count(inner_w) as u16 + 2).min(area.height / 2 + 4);
    let [top, bottom] = Layout::vertical([Constraint::Length(needed), Constraint::Min(4)]).areas(area);

    f.render_widget(para.block(pane("Panel (schematic, grouped by manual section)", true)), top);
    draw_detail(f, app, bottom, false);
}

// ───────────────────────────── browse view ─────────────────────────────

fn draw_browse(f: &mut Frame, app: &mut App, area: Rect) {
    // Unit drawing at the bottom, if enabled and the panes keep a usable height.
    let area = if app.show_unit && area.height >= UNIT_H + 10 {
        let [panes, bottom] = Layout::vertical([Constraint::Min(10), Constraint::Length(UNIT_H)]).areas(area);
        draw_unit(f, app, bottom, app.current());
        panes
    } else {
        area
    };
    let [left, mid, right] =
        Layout::horizontal([Constraint::Length(32), Constraint::Length(38), Constraint::Min(30)]).areas(area);

    // Sections with group headers.
    let mut items = Vec::new();
    let mut selected_row = 0;
    let mut last_group = "";
    for (pos, &s) in app.cat.section_order.iter().enumerate() {
        let g = app.cat.group_title(s);
        if g != last_group {
            items.push(ListItem::new(Line::styled(g.to_string(), Style::new().fg(DIM).bold())));
            last_group = g;
        }
        if pos == app.sec_pos {
            selected_row = items.len();
        }
        items.push(ListItem::new(format!("  {}", app.cat.sections[s].title)));
    }
    app.sec_state.select(Some(selected_row));
    let searching = app.search.is_some();
    let list = List::new(items)
        .block(pane("Sections", app.focus == Focus::Sections && !searching))
        .highlight_style(if searching { Style::new() } else { highlight() });
    f.render_stateful_widget(list, left, &mut app.sec_state);

    if let Some(s) = app.search.as_mut() {
        let items: Vec<ListItem> = s
            .results
            .iter()
            .map(|&e| {
                let entry = &app.cat.entries[e];
                let sec = &app.cat.sections[app.cat.section_of(e)].title;
                ListItem::new(vec![Line::raw(entry.name.clone()), Line::styled(format!("  {sec}"), Style::new().fg(DIM))])
            })
            .collect();
        let title = format!("{} result{}", s.results.len(), if s.results.len() == 1 { "" } else { "s" });
        let list = List::new(items).block(pane(&title, true)).highlight_style(highlight());
        f.render_stateful_widget(list, mid, &mut s.state);
    } else {
        let items: Vec<ListItem> = app
            .section_entries()
            .iter()
            .map(|&e| {
                let entry = &app.cat.entries[e];
                let mut spans = vec![Span::raw(entry.name.clone())];
                if let Some(d) = &entry.display {
                    spans.push(Span::styled(format!("  {d}"), Style::new().fg(LED)));
                }
                ListItem::new(Line::from(spans))
            })
            .collect();
        let title = app.cat.sections[app.section()].title.clone();
        let list = List::new(items).block(pane(&title, app.focus == Focus::Entries)).highlight_style(highlight());
        f.render_stateful_widget(list, mid, &mut app.ent_state);
    }

    let focused = app.focus == Focus::Detail && !searching;
    draw_detail(f, app, right, focused);
}

// ───────────────────────────── detail ─────────────────────────────

fn draw_detail(f: &mut Frame, app: &mut App, area: Rect, focused: bool) {
    let Some(e) = app.current() else {
        let msg = if app.search.as_ref().is_some_and(|s| !s.query.is_empty()) { "No matches." } else { "Type to search…" };
        f.render_widget(Paragraph::new(msg).fg(DIM).block(pane("Detail", focused)), area);
        return;
    };
    // Links are selectable only while the Detail pane has focus in Browse view.
    let picking = app.view == View::Browse && app.focus == Focus::Detail && app.search.is_none();
    let (lines, sel_line) = detail_lines(app, e, if picking { app.link_sel } else { None }, picking);
    let inner_w = area.width.saturating_sub(2).max(1);
    let visible = area.height.saturating_sub(2);
    // Keep the selected link on screen (measured in wrapped rows).
    if let Some(li) = sel_line {
        let row = Paragraph::new(lines[..li].to_vec()).wrap(Wrap { trim: false }).line_count(inner_w) as u16;
        if row < app.detail_scroll {
            app.detail_scroll = row;
        } else if row >= app.detail_scroll + visible {
            app.detail_scroll = row + 1 - visible;
        }
    }
    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    let total = para.line_count(inner_w) as u16;
    app.detail_scroll = app.detail_scroll.min(total.saturating_sub(visible));
    let title = if total > visible { "Detail (J/K to scroll)" } else { "Detail" };
    f.render_widget(para.scroll((app.detail_scroll, 0)).block(pane(title, focused)), area);
}

fn heading(text: &str) -> Line<'static> {
    Line::styled(text.to_string(), Style::new().fg(ACCENT).bold())
}

const KNOBS: &[&str] = &["tempo", "volume", "ctrl1", "ctrl2", "ctrl3", "pitch", "start", "end", "level"];

/// One key-press step as an instruction, e.g. "Holding SHIFT, press F#·P.ENV", plus the
/// display code to look for when the step selects a menu item.
fn step_text<'a>(app: &App, fr: &KeyFrame, display: Option<&'a str>) -> (String, Option<&'a str>) {
    let names: Vec<&str> = fr.active.iter().map(|id| key_label(app, id)).collect();
    let target = match names.as_slice() {
        [one] => one.to_string(),
        [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
        [] => String::new(),
    };
    let verb = if fr.active.iter().all(|id| KNOBS.contains(id)) { "turn" } else { "press" };
    let mut s = if fr.held.is_empty() {
        let mut v = verb.to_string();
        v[..1].make_ascii_uppercase();
        format!("{v} {target}")
    } else {
        let held: Vec<&str> = fr.held.iter().map(|id| key_label(app, id)).collect();
        format!("Holding {}, {verb} {target}", held.join(" + "))
    };
    // Selecting a menu item with TEMPO/VALUE: say what to look for.
    let code = (fr.held.is_empty() && fr.active == ["tempo"]).then_some(display).flatten();
    if code.is_some() {
        s.push_str(" until the display shows ");
    }
    (s, code)
}

/// A display code styled like the unit's LED.
fn led(code: &str) -> Span<'static> {
    Span::styled(format!(" {code} "), Style::new().fg(LED).bg(Color::Rgb(25, 0, 0)).bold())
}

/// Detail lines for an entry, plus the line index of the selected "Used in" link.
fn detail_lines(app: &App, idx: usize, link_sel: Option<usize>, picking: bool) -> (Vec<Line<'static>>, Option<usize>) {
    let mut sel_line = None;
    let cat = &app.cat;
    let e = &cat.entries[idx];
    let sec = cat.section_of(idx);
    let mut out = vec![
        Line::styled(e.name.clone(), Style::new().fg(Color::White).bold().add_modifier(Modifier::UNDERLINED)),
        Line::styled(format!("{} › {}", cat.group_title(sec), cat.sections[sec].title), Style::new().fg(DIM)),
        Line::raw(""),
    ];

    if let Some(c) = &e.combo {
        out.push(Line::from(vec![Span::styled("Do       ", Style::new().fg(DIM)), Span::styled(c.clone(), Style::new().fg(KEY).bold())]));
    }
    if let Some(d) = &e.display {
        out.push(Line::from(vec![
            Span::styled("Display  ", Style::new().fg(DIM)),
            led(d),
        ]));
    }
    if e.combo.is_some() || e.display.is_some() {
        out.push(Line::raw(""));
    }

    let frames = &cat.sequences[idx];
    if !frames.is_empty() {
        out.push(heading("Key presses"));
        // Only highlight a step if the stepper belongs to this entry.
        let step = if app.anim_entry == Some(idx) { app.step } else { None };
        let first_code = cat.display_codes(idx).first().copied();
        for (i, fr) in frames.iter().enumerate() {
            let (text, code) = step_text(app, fr, first_code);
            let mut spans = if step == Some(i) {
                vec![
                    Span::styled(format!(" >{}. ", i + 1), Style::new().fg(unit::PURPLE).bold()),
                    Span::styled(text, Style::new().fg(Color::Black).bg(unit::PURPLE).bold()),
                ]
            } else {
                vec![Span::styled(format!("  {}. ", i + 1), Style::new().fg(KEY)), Span::raw(text)]
            };
            spans.extend(code.map(led));
            out.push(Line::from(spans));
        }
        let hint = if step.is_some() {
            "  Enter/n next · p back"
        } else {
            "  Enter in this pane (or n) to step through on the unit"
        };
        out.push(Line::styled(hint, Style::new().fg(DIM)));
        out.push(Line::raw(""));
    }

    if !e.summary.is_empty() {
        out.push(Line::raw(e.summary.clone()));
        out.push(Line::raw(""));
    }

    if !e.values.is_empty() {
        out.push(heading(if e.id.is_some() { "Functions" } else { "Values" }));
        let w = e.values.iter().map(|(v, _)| v.chars().count()).max().unwrap_or(0).min(30);
        for (v, d) in &e.values {
            let mut spans = vec![Span::raw("  "), Span::styled(format!("{v:<w$}"), Style::new().fg(VAL))];
            if !d.is_empty() {
                spans.push(Span::raw(format!("  {d}")));
            }
            out.push(Line::from(spans));
        }
        out.push(Line::raw(""));
    }

    if !e.steps.is_empty() {
        out.push(heading("Steps"));
        for (i, s) in e.steps.iter().enumerate() {
            out.push(Line::from(vec![Span::styled(format!("  {}. ", i + 1), Style::new().fg(KEY)), Span::raw(s.clone())]));
        }
        out.push(Line::raw(""));
    }

    if !e.notes.is_empty() {
        out.push(heading("Notes"));
        for n in &e.notes {
            out.push(Line::from(vec![Span::styled("  • ", Style::new().fg(DIM)), Span::raw(n.clone())]));
        }
        out.push(Line::raw(""));
    }

    let related = &cat.related[idx];
    if !related.is_empty() {
        out.push(heading("Used in"));
        for (i, &r) in related.iter().enumerate() {
            let re = &cat.entries[r];
            let selected = link_sel == Some(i);
            let mut spans = if selected {
                sel_line = Some(out.len());
                vec![
                    Span::styled("  > ", Style::new().fg(unit::PURPLE).bold()),
                    Span::styled(re.name.clone(), Style::new().fg(Color::Black).bg(unit::PURPLE).bold()),
                ]
            } else {
                vec![Span::styled("  • ", Style::new().fg(DIM)), Span::raw(re.name.clone())]
            };
            if let Some(c) = &re.combo {
                spans.push(Span::styled(format!("  {c}"), Style::new().fg(KEY)));
            }
            out.push(Line::from(spans));
        }
        let hint = if picking {
            "  j/k select · Enter open · Backspace back"
        } else if app.view == View::Panel {
            "  Enter to open this control and pick one"
        } else {
            "  l / → into this pane to pick one"
        };
        out.push(Line::styled(hint, Style::new().fg(DIM)));
        out.push(Line::raw(""));
    }

    if let Some(p) = e.page {
        out.push(Line::styled(format!("Owner's Manual p. {p}"), Style::new().fg(DIM).italic()));
    }
    (out, sel_line)
}

// ───────────────────────────── help ─────────────────────────────

fn draw_help(f: &mut Frame) {
    let area = f.area();
    let w = 58.min(area.width);
    let h = 25.min(area.height);
    let rect = Rect::new(area.x + (area.width - w) / 2, area.y + (area.height - h) / 2, w, h);
    let k = |key: &str, d: &str| {
        Line::from(vec![Span::styled(format!("  {key:<14}"), Style::new().fg(KEY)), Span::raw(d.to_string())])
    };
    let mut lines = vec![
        Line::raw(""),
        k("Tab", "switch Panel / Browse"),
        k("/", "search everything (fuzzy)"),
        k("v", "show/hide the drawing of the unit"),
        k("Backspace", "back to the previous entry"),
        k("Enter / n", "next key press (Enter: in Detail)"),
        k("p", "previous key-press step"),
        k("↑↓←→ / hjkl", "move"),
        k("Enter", "panel: open control · search: open"),
        k("J/K PgDn/PgUp", "scroll the detail pane"),
    ];
    if crate::WEB {
        lines.push(k("Esc", "close search"));
    } else {
        lines.push(k("Esc", "close search · quit"));
        lines.push(k("Ctrl-L", "repaint the screen"));
        lines.push(k("q", "quit"));
    }
    lines.extend([
        Line::raw(""),
        Line::styled("  Notation", Style::new().fg(ACCENT).bold()),
        Line::raw("  SHIFT + X   hold SHIFT, press X"),
        Line::raw("  A → B       press A, then B"),
        Line::raw("  C# (ENTER)  step key by note name + printed label"),
        Line::raw("  ENTER/EXIT  are the C# and C step keys"),
        Line::raw(""),
        Line::styled("  Data: Roland P-6 Owner's Manual (fw 1.02)", Style::new().fg(DIM)),
        Line::styled("  press any key to close", Style::new().fg(DIM)),
    ]);
    f.render_widget(Clear, rect);
    f.render_widget(Paragraph::new(lines).block(pane("Help", true)), rect);
}
