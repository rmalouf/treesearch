//! Terminal browser for refining treesearch queries.
//!
//! Usage: `treesearch-tui GLOB [QUERY_FILE]`
//!
//! Edit a query, run it against the corpus, skim the hits as they stream in,
//! and open any hit to see the whole tree. Re-running cancels the previous search.

use std::path::PathBuf;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{env, fs, io, process, thread};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::DefaultTerminal;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListState, Paragraph, Wrap};
use treesearch::{Match, Progress, Treebank, compile_query};
use tui_textarea::TextArea;

/// Only this many hits are kept; the rest are counted.
const MAX_HITS: usize = 5000;
/// Column the keyword is aligned to, as a fraction of the list width.
const KWIC_SPLIT: f32 = 0.4;

#[derive(Default)]
struct Results {
    hits: Vec<Match>,
    total: usize,
    done: bool,
    error: Option<String>,
}

struct Search {
    progress: Arc<Progress>,
    results: Arc<Mutex<Results>>,
}

#[derive(PartialEq)]
enum View {
    Editor,
    Results,
}

struct App {
    treebank: Treebank,
    query_path: Option<PathBuf>,
    editor: TextArea<'static>,
    view: View,
    search: Option<Search>,
    selected: usize,
    show_detail: bool,
    focus_detail: bool,
    detail_scroll: u16,
    status: Option<String>,
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let Some(glob) = args.get(1) else {
        eprintln!("usage: treesearch-tui GLOB [QUERY_FILE]");
        process::exit(2);
    };
    let treebank = Treebank::from_glob(glob).unwrap_or_else(|e| {
        eprintln!("bad glob {glob}: {e}");
        process::exit(2);
    });
    let query_path = args.get(2).map(PathBuf::from);
    let text = match &query_path {
        Some(p) => fs::read_to_string(p).unwrap_or_default(),
        None => String::new(),
    };

    let mut app = App {
        treebank,
        query_path,
        editor: TextArea::from(text.lines()),
        view: View::Editor,
        search: None,
        selected: 0,
        show_detail: false,
        focus_detail: false,
        detail_scroll: 0,
        status: None,
    };

    let mut terminal = ratatui::init();
    let result = app.run(&mut terminal);
    ratatui::restore();
    if let Some(s) = &app.search {
        s.progress.cancel();
    }
    result
}

impl App {
    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.draw(f))?;
            if event::poll(Duration::from_millis(50))?
                && let Event::Key(key) = event::read()?
                && key.kind == event::KeyEventKind::Press
                && !self.handle_key(key)
            {
                return Ok(());
            }
        }
    }

    /// Returns false when the app should exit.
    fn handle_key(&mut self, key: KeyEvent) -> bool {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl && matches!(key.code, KeyCode::Char('q') | KeyCode::Char('c')) {
            return false;
        }
        match self.view {
            View::Editor => match key.code {
                KeyCode::Char('r') if ctrl => self.run_query(),
                KeyCode::Char('s') if ctrl => self.save_query(),
                KeyCode::Esc if self.search.is_some() => self.view = View::Results,
                _ => {
                    self.editor.input(key);
                }
            },
            View::Results => match key.code {
                KeyCode::Char('q') => return false,
                KeyCode::Char('e') => self.view = View::Editor,
                KeyCode::Esc if self.show_detail => self.close_detail(),
                KeyCode::Esc => self.view = View::Editor,
                KeyCode::Enter if self.show_detail => self.close_detail(),
                KeyCode::Enter if self.hit_count() > 0 => self.show_detail = true,
                KeyCode::Tab if self.show_detail => self.focus_detail = !self.focus_detail,
                _ if self.show_detail && self.focus_detail => self.scroll_detail(key.code),
                _ => self.move_selection(key.code),
            },
        }
        true
    }

    fn close_detail(&mut self) {
        self.show_detail = false;
        self.focus_detail = false;
        self.detail_scroll = 0;
    }

    fn scroll_detail(&mut self, code: KeyCode) {
        self.detail_scroll = match code {
            KeyCode::Up | KeyCode::Char('k') => self.detail_scroll.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => self.detail_scroll + 1,
            KeyCode::PageUp => self.detail_scroll.saturating_sub(10),
            KeyCode::PageDown => self.detail_scroll + 10,
            _ => self.detail_scroll,
        };
    }

    fn move_selection(&mut self, code: KeyCode) {
        let n = self.hit_count();
        let last = n.saturating_sub(1);
        self.selected = match code {
            KeyCode::Up | KeyCode::Char('k') => self.selected.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => (self.selected + 1).min(last),
            KeyCode::PageUp => self.selected.saturating_sub(20),
            KeyCode::PageDown => (self.selected + 20).min(last),
            KeyCode::Home | KeyCode::Char('g') => 0,
            KeyCode::End | KeyCode::Char('G') => last,
            _ => self.selected,
        };
        self.detail_scroll = 0;
    }

    fn hit_count(&self) -> usize {
        self.search
            .as_ref()
            .map_or(0, |s| s.results.lock().unwrap().hits.len())
    }

    fn run_query(&mut self) {
        let pattern = match compile_query(&self.editor.lines().join("\n")) {
            Ok(p) => p,
            Err(e) => {
                // Pest errors span several lines; the status bar has one.
                self.status = Some(
                    e.to_string()
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" "),
                );
                return;
            }
        };
        if let Some(old) = self.search.take() {
            old.progress.cancel();
        }
        let progress = Arc::new(Progress::default());
        let results = Arc::new(Mutex::new(Results::default()));
        let (treebank, prog, res) = (self.treebank.clone(), progress.clone(), results.clone());
        thread::spawn(move || {
            // Infallible: the pattern is already compiled.
            for item in treebank.search_with(pattern, false, prog).unwrap() {
                let mut r = res.lock().unwrap();
                match item {
                    Ok(m) => {
                        r.total += 1;
                        if r.hits.len() < MAX_HITS {
                            r.hits.push(m);
                        }
                    }
                    Err(e) => r.error = Some(e.to_string()),
                }
            }
            res.lock().unwrap().done = true;
        });
        self.search = Some(Search { progress, results });
        self.selected = 0;
        self.close_detail();
        self.status = None;
        self.view = View::Results;
    }

    fn save_query(&mut self) {
        let Some(path) = &self.query_path else {
            self.status = Some("no query file given on the command line".into());
            return;
        };
        self.status = Some(
            match fs::write(path, self.editor.lines().join("\n") + "\n") {
                Ok(()) => format!("saved {}", path.display()),
                Err(e) => format!("{}: {e}", path.display()),
            },
        );
    }

    fn draw(&mut self, frame: &mut Frame) {
        let [main, status] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());
        match self.view {
            View::Editor => {
                self.editor.set_block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Query — Ctrl-R run · Ctrl-S save · Esc results · Ctrl-Q quit "),
                );
                frame.render_widget(&self.editor, main);
            }
            View::Results if self.show_detail => {
                let [list, detail] =
                    Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .areas(main);
                self.draw_list(frame, list);
                self.draw_detail(frame, detail);
            }
            View::Results => self.draw_list(frame, main),
        }
        frame.render_widget(self.status_line(), status);
    }

    fn status_line(&self) -> Line<'_> {
        let mut text = match &self.search {
            None => String::from("no search yet"),
            Some(s) => {
                let r = s.results.lock().unwrap();
                let p = &s.progress;
                let mut t = format!(
                    "{} {}/{} files · {} trees · {} hits",
                    if r.done { "done" } else { "running" },
                    p.files_done.load(Relaxed),
                    p.files_total.load(Relaxed),
                    p.trees.load(Relaxed),
                    r.total,
                );
                if r.total > MAX_HITS {
                    t += &format!(" (showing first {MAX_HITS})");
                }
                if let Some(e) = &r.error {
                    t += &format!(" · {e}");
                }
                t
            }
        };
        if let Some(msg) = &self.status {
            text = format!("{msg} | {text}");
        }
        Line::styled(text, Style::default().reversed())
    }

    fn draw_list(&mut self, frame: &mut Frame, area: Rect) {
        let title = if self.show_detail {
            " Hits — Tab focus detail · Esc close · e edit "
        } else {
            " Hits — Enter detail · e edit · q quit "
        };
        let block = Block::default().borders(Borders::ALL).title(title);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let Some(search) = &self.search else { return };
        let results = search.results.lock().unwrap();
        let height = inner.height as usize;
        let offset = self.selected.saturating_sub(height.saturating_sub(1));
        let visible = results.hits.iter().skip(offset).take(height);
        let items: Vec<Line> = visible.map(|m| kwic(m, inner.width as usize)).collect();
        let mut state = ListState::default().with_selected(Some(self.selected - offset));
        let list = List::new(items).highlight_style(Style::default().reversed());
        frame.render_stateful_widget(list, inner, &mut state);
    }

    fn draw_detail(&mut self, frame: &mut Frame, area: Rect) {
        let Some(search) = &self.search else { return };
        let results = search.results.lock().unwrap();
        let Some(m) = results.hits.get(self.selected) else {
            return;
        };
        let border = if self.focus_detail {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border)
            .title(" Tree ");
        let para = Paragraph::new(detail_lines(m))
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((self.detail_scroll, 0));
        frame.render_widget(para, area);
    }
}

fn bound_style(m: &Match, id: usize) -> Style {
    if m.bindings.values().any(|&w| w == id) {
        Style::default().fg(Color::Yellow).bold()
    } else {
        Style::default()
    }
}

/// One-line keyword-in-context view: the lowest-index bound word is the keyword,
/// aligned to a fixed column; context is trimmed word by word to fit.
fn kwic(m: &Match, width: usize) -> Line<'static> {
    let tree = &m.tree;
    let forms: Vec<String> = tree
        .words
        .iter()
        .map(|w| String::from_utf8_lossy(&tree.string_pool.resolve(w.form)).into_owned())
        .collect();
    let kw = m.bindings.values().copied().min().unwrap_or(0);
    let left_width = (width as f32 * KWIC_SPLIT) as usize;
    let right_width = width.saturating_sub(left_width + forms[kw].chars().count() + 2);

    let mut left = vec![];
    let mut used = 0;
    for id in (0..kw).rev() {
        let w = forms[id].chars().count() + 1;
        if used + w > left_width {
            break;
        }
        used += w;
        left.push(Span::styled(forms[id].clone() + " ", bound_style(m, id)));
    }
    left.push(Span::raw(" ".repeat(left_width - used)));
    left.reverse();

    let mut spans = left;
    spans.push(Span::styled(forms[kw].clone(), bound_style(m, kw)));
    used = 0;
    for (id, form) in forms.iter().enumerate().skip(kw + 1) {
        let w = form.chars().count() + 1;
        if used + w > right_width {
            break;
        }
        used += w;
        spans.push(Span::styled(" ".to_string() + form, bound_style(m, id)));
    }
    Line::from(spans)
}

/// Full view of one hit: metadata, bindings, and the CoNLL-U table with bound rows highlighted.
fn detail_lines(m: &Match) -> Vec<Line<'static>> {
    let tree = &m.tree;
    let s = |sym| String::from_utf8_lossy(&tree.string_pool.resolve(sym)).into_owned();
    let mut lines = vec![];
    if let Some(text) = &tree.sentence_text {
        lines.push(Line::from(text.clone()));
    }
    let mut meta: Vec<_> = tree.metadata.iter().filter(|(k, _)| *k != "text").collect();
    meta.sort();
    for (k, v) in meta {
        lines.push(Line::styled(format!("# {k} = {v}"), Style::default().dim()));
    }
    let mut bindings: Vec<_> = m.bindings.iter().collect();
    bindings.sort();
    let spans: Vec<Span> = bindings
        .iter()
        .flat_map(|&(name, &id)| {
            [
                Span::raw(format!("{name}=")),
                Span::styled(s(tree.words[id].form) + "  ", bound_style(m, id)),
            ]
        })
        .collect();
    lines.push(Line::from(spans));
    lines.push(Line::raw(""));

    let rows: Vec<[String; 5]> = tree
        .words
        .iter()
        .map(|w| {
            [
                w.token_id.to_string(),
                s(w.form),
                s(w.lemma),
                s(w.upos),
                s(w.deprel),
            ]
        })
        .collect();
    let mut widths = [0; 5];
    for row in &rows {
        for (w, cell) in widths.iter_mut().zip(row) {
            *w = (*w).max(cell.chars().count());
        }
    }
    for (w, row) in tree.words.iter().zip(&rows) {
        let head = w
            .head
            .map_or("0".to_string(), |h| tree.words[h].token_id.to_string());
        let cells: Vec<String> = row
            .iter()
            .zip(widths)
            .map(|(cell, width)| format!("{cell:<width$}"))
            .collect();
        let text = format!("{} {}", cells.join("  "), head);
        lines.push(Line::styled(text, bound_style(m, w.id)));
    }
    lines
}
