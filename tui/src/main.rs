//! Terminal browser for refining treesearch queries.
//!
//! Usage: `treesearch-tui GLOB [QUERY_FILE]`
//!
//! Three panes, all visible: the query, the hits streaming in as they are found,
//! and the dependency tree of the selected hit. Re-running cancels the previous
//! search; only the first `MAX_HITS` hits are kept, the rest are counted.

use std::path::PathBuf;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{env, fs, io, process, thread};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use pest::error::LineColLocation;
use ratatui::DefaultTerminal;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListState, Paragraph, Wrap};
use treesearch::{Match, Pattern, Progress, QueryError, Tree, Treebank, WordId, compile_query};
use tui_textarea::{CursorMove, TextArea};

/// Only this many hits are kept; the rest are counted.
const MAX_HITS: usize = 5000;
/// Column the keyword is aligned to, as a fraction of the context width.
const KWIC_SPLIT: f32 = 0.4;
/// Terminals at least this wide get the tree beside the query and hits instead of below.
const WIDE_LAYOUT: u16 = 120;
const VAR_COLORS: [Color; 6] = [
    Color::Yellow,
    Color::Cyan,
    Color::Magenta,
    Color::Green,
    Color::Red,
    Color::Blue,
];

#[derive(Default)]
struct Results {
    hits: Vec<Match>,
    total: usize,
    done: bool,
    error: Option<String>,
}

struct Search {
    /// Pattern variables in declaration order.
    vars: Vec<String>,
    progress: Arc<Progress>,
    results: Arc<Mutex<Results>>,
}

#[derive(Clone, Copy, PartialEq)]
enum Focus {
    Query,
    Hits,
    Tree,
}

struct App {
    treebank: Treebank,
    query_path: Option<PathBuf>,
    editor: TextArea<'static>,
    search: Option<Search>,
    focus: Focus,
    selected: usize,
    tree_scroll: u16,
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
        search: None,
        focus: Focus::Query,
        selected: 0,
        tree_scroll: 0,
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
        match key.code {
            KeyCode::Char('q') if ctrl => return false,
            KeyCode::Char('c') if ctrl => {
                if let Some(s) = &self.search {
                    s.progress.cancel();
                }
            }
            KeyCode::Char('r') if ctrl => self.run_query(),
            KeyCode::Char('s') if ctrl => self.save_query(),
            KeyCode::Tab => self.cycle_focus(1),
            KeyCode::BackTab => self.cycle_focus(2),
            KeyCode::Esc => {
                self.focus = if self.focus == Focus::Query {
                    Focus::Hits
                } else {
                    Focus::Query
                }
            }
            _ => match self.focus {
                Focus::Query => {
                    self.editor.input(key);
                }
                Focus::Hits => match key.code {
                    KeyCode::Char('q') => return false,
                    KeyCode::Enter => self.focus = Focus::Tree,
                    code => self.move_selection(code),
                },
                Focus::Tree => match key.code {
                    KeyCode::Char('q') => return false,
                    KeyCode::Char('n') => self.move_selection(KeyCode::Down),
                    KeyCode::Char('p') => self.move_selection(KeyCode::Up),
                    code => self.scroll_tree(code),
                },
            },
        }
        true
    }

    fn cycle_focus(&mut self, by: usize) {
        const ORDER: [Focus; 3] = [Focus::Query, Focus::Hits, Focus::Tree];
        let i = ORDER.iter().position(|&f| f == self.focus).unwrap();
        self.focus = ORDER[(i + by) % 3];
    }

    fn scroll_tree(&mut self, code: KeyCode) {
        self.tree_scroll = match code {
            KeyCode::Up | KeyCode::Char('k') => self.tree_scroll.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => self.tree_scroll + 1,
            KeyCode::PageUp => self.tree_scroll.saturating_sub(10),
            KeyCode::PageDown => self.tree_scroll + 10,
            KeyCode::Home | KeyCode::Char('g') => 0,
            _ => self.tree_scroll,
        };
    }

    fn move_selection(&mut self, code: KeyCode) {
        let last = self.hit_count().saturating_sub(1);
        self.selected = match code {
            KeyCode::Up | KeyCode::Char('k') => self.selected.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => (self.selected + 1).min(last),
            KeyCode::PageUp => self.selected.saturating_sub(20),
            KeyCode::PageDown => (self.selected + 20).min(last),
            KeyCode::Home | KeyCode::Char('g') => 0,
            KeyCode::End | KeyCode::Char('G') => last,
            _ => self.selected,
        };
        self.tree_scroll = 0;
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
                if let QueryError::ParseError(pe) = &e {
                    let (line, col) = match pe.line_col {
                        LineColLocation::Pos(p) | LineColLocation::Span(p, _) => p,
                    };
                    self.editor
                        .move_cursor(CursorMove::Jump(line as u16 - 1, col as u16 - 1));
                    self.focus = Focus::Query;
                }
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
        let vars = pattern_vars(&pattern);
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
        self.search = Some(Search {
            vars,
            progress,
            results,
        });
        self.selected = 0;
        self.tree_scroll = 0;
        self.status = None;
        self.focus = Focus::Hits;
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
        let halves = [Constraint::Percentage(50), Constraint::Percentage(50)];
        // Wide: query over hits on the left, tree full height on the right.
        let [left, tree] = if main.width >= WIDE_LAYOUT {
            Layout::horizontal(halves).areas(main)
        } else {
            Layout::vertical([Constraint::Fill(3), Constraint::Fill(2)]).areas(main)
        };
        let query_h = (self.editor.lines().len() as u16 + 2).clamp(3, left.height * 2 / 5);
        let [query, hits] =
            Layout::vertical([Constraint::Length(query_h), Constraint::Fill(1)]).areas(left);
        self.draw_query(frame, query);
        self.draw_hits(frame, hits);
        self.draw_tree(frame, tree);
        frame.render_widget(self.status_line(), status);
    }

    fn pane(&self, title: String, focus: Focus) -> Block<'static> {
        let style = if self.focus == focus {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        Block::default()
            .borders(Borders::ALL)
            .border_style(style)
            .title(title)
    }

    fn draw_query(&mut self, frame: &mut Frame, area: Rect) {
        let name = self
            .query_path
            .as_ref()
            .map_or(String::new(), |p| format!(" · {}", p.display()));
        let block = self.pane(format!(" Query{name} "), Focus::Query);
        let cursor = if self.focus == Focus::Query {
            Style::default().reversed()
        } else {
            Style::default()
        };
        self.editor.set_block(block);
        self.editor.set_cursor_style(cursor);
        frame.render_widget(&self.editor, area);
    }

    fn draw_hits(&mut self, frame: &mut Frame, area: Rect) {
        let block = self.pane(" Hits ".into(), Focus::Hits);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let Some(search) = &self.search else { return };
        let results = search.results.lock().unwrap();
        let height = inner.height as usize;
        let offset = self.selected.saturating_sub(height.saturating_sub(1));
        let rows: Vec<Line> = results
            .hits
            .iter()
            .skip(offset)
            .take(height)
            .map(|m| hit_row(m, &search.vars, inner.width as usize))
            .collect();
        let mut state = ListState::default().with_selected(Some(self.selected - offset));
        let list = List::new(rows).highlight_style(Style::default().reversed());
        frame.render_stateful_widget(list, inner, &mut state);
    }

    fn draw_tree(&mut self, frame: &mut Frame, area: Rect) {
        let mut title = " Tree ".to_string();
        let mut lines = vec![];
        if let Some(s) = &self.search {
            let r = s.results.lock().unwrap();
            if let Some(m) = r.hits.get(self.selected) {
                let id = m.tree.metadata.get("sent_id").map_or("", String::as_str);
                title = format!(" Tree · {}/{} · {id} ", self.selected + 1, r.hits.len());
                lines = tree_lines(m, &s.vars, area.width.saturating_sub(2) as usize);
            }
        }
        let para = Paragraph::new(lines)
            .block(self.pane(title, Focus::Tree))
            .wrap(Wrap { trim: false })
            .scroll((self.tree_scroll, 0));
        frame.render_widget(para, area);
    }

    fn status_line(&self) -> Line<'_> {
        let mut parts = vec![];
        if let Some(msg) = &self.status {
            parts.push(msg.clone());
        }
        if let Some(s) = &self.search {
            let r = s.results.lock().unwrap();
            let p = &s.progress;
            let mut t = format!(
                "{} {}/{} files · {} trees · {} hits",
                if p.is_cancelled() {
                    "interrupted"
                } else if r.done {
                    "done"
                } else {
                    "running"
                },
                p.files_done.load(Relaxed),
                p.files_total.load(Relaxed),
                p.trees.load(Relaxed),
                r.total,
            );
            if r.total > MAX_HITS {
                t += &format!(" (first {MAX_HITS} kept)");
            }
            if let Some(e) = &r.error {
                t += &format!(" · {e}");
            }
            parts.push(t);
        }
        parts.push("^R run · ^C stop · ^S save · Tab focus · n/p next/prev hit · ^Q quit".into());
        Line::styled(parts.join("  |  "), Style::default().reversed())
    }
}

/// Pattern variables in declaration order: MATCH block first, then OPTIONAL blocks.
fn pattern_vars(p: &Pattern) -> Vec<String> {
    let mut vars = p.match_pattern.var_names.clone();
    for opt in &p.optional_patterns {
        for v in &opt.var_names {
            if !vars.contains(v) {
                vars.push(v.clone());
            }
        }
    }
    vars
}

fn var_style(i: usize) -> Style {
    Style::default().fg(VAR_COLORS[i % VAR_COLORS.len()]).bold()
}

/// Index of the first variable bound to `id`, if any.
fn var_of(m: &Match, vars: &[String], id: WordId) -> Option<usize> {
    vars.iter().position(|v| m.bindings.get(v) == Some(&id))
}

fn word_style(m: &Match, vars: &[String], id: WordId) -> Style {
    var_of(m, vars, id).map_or(Style::default(), var_style)
}

fn form(tree: &Tree, id: WordId) -> String {
    String::from_utf8_lossy(&tree.string_pool.resolve(tree.words[id].form)).into_owned()
}

fn pad(s: &str, width: usize) -> String {
    let s: String = s.chars().take(width).collect();
    format!("{s:<width$}")
}

/// One hit as keyword-in-context: the first variable's word aligned to a fixed
/// column, bound words colored by variable.
fn hit_row(m: &Match, vars: &[String], width: usize) -> Line<'static> {
    let tree = &m.tree;
    let mut spans = vec![];
    let forms: Vec<String> = (0..tree.words.len()).map(|i| form(tree, i)).collect();
    let kw = vars
        .first()
        .and_then(|v| m.bindings.get(v).copied())
        .unwrap_or(0);
    let left_width = (width as f32 * KWIC_SPLIT) as usize;
    let right_width = width.saturating_sub(left_width + forms[kw].chars().count());

    let mut left = vec![];
    let mut used = 0;
    for id in (0..kw).rev() {
        let w = forms[id].chars().count() + 1;
        if used + w > left_width {
            break;
        }
        used += w;
        left.push(Span::styled(
            forms[id].clone() + " ",
            word_style(m, vars, id),
        ));
    }
    left.push(Span::raw(" ".repeat(left_width - used)));
    left.reverse();
    spans.extend(left);

    spans.push(Span::styled(forms[kw].clone(), word_style(m, vars, kw)));
    used = 0;
    for (id, f) in forms.iter().enumerate().skip(kw + 1) {
        let w = f.chars().count() + 1;
        if used + w > right_width {
            break;
        }
        used += w;
        spans.push(Span::styled(" ".to_string() + f, word_style(m, vars, id)));
    }
    Line::from(spans)
}

/// Full view of one hit: sentence, metadata, then the tree with bound words tagged.
fn tree_lines(m: &Match, vars: &[String], width: usize) -> Vec<Line<'static>> {
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
    lines.push(Line::raw(""));

    // One row per word in sentence order; the head is named by id and form.
    let tag_w = vars.iter().map(|v| v.chars().count()).max().unwrap_or(0);
    let cells: Vec<[String; 5]> = tree
        .words
        .iter()
        .map(|w| {
            let head = w.head.map_or(String::new(), |h| form(tree, h));
            [s(w.form), s(w.lemma), s(w.upos), s(w.deprel), head]
        })
        .collect();
    let mut widths = [0; 5];
    for row in &cells {
        for (w, cell) in widths.iter_mut().zip(row) {
            *w = (*w).max(cell.chars().count());
        }
    }
    for (i, w) in tree.words.iter().enumerate() {
        let style = word_style(m, vars, i);
        let tag = var_of(m, vars, i).map_or("", |v| vars[v].as_str());
        let [form, lemma, upos, deprel, head] = &cells[i];
        let head_id = w.head.map_or(0, |h| tree.words[h].token_id);
        let head_style = w.head.map_or(Style::default(), |h| word_style(m, vars, h));
        let mut line = vec![
            Span::styled(pad(tag, tag_w), style),
            Span::raw(format!(" {:>3}  ", w.token_id)),
            Span::styled(pad(form, widths[0]), style),
            Span::raw(format!(
                "  {}  {}  {}  {head_id:>3} ",
                pad(lemma, widths[1]),
                pad(upos, widths[2]),
                pad(deprel, widths[3]),
            )),
            Span::styled(pad(head, widths[4]), head_style),
            Span::raw("  "),
        ];
        let feats: Vec<String> = w
            .feats
            .iter()
            .map(|&(k, v)| format!("{}={}", s(k), s(v)))
            .collect();
        // Cut at the pane edge so rows don't wrap.
        let used: usize = line.iter().map(|sp| sp.content.chars().count()).sum();
        let feats: String = feats
            .join("|")
            .chars()
            .take(width.saturating_sub(used))
            .collect();
        line.push(Span::styled(feats, Style::default().dim()));
        lines.push(Line::from(line));
    }
    lines
}
