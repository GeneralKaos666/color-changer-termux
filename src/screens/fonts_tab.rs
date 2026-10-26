use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph},
};

use crate::app::Command;
use crate::chrome;
use crate::fonts::{apply, prompt, scan};
use crate::model::font_slot::FontSlot;
use crate::paths::Paths;
use crate::runtime::Msg;
use crate::screens::ScreenAction;
use crate::ui::dialog::{Dialog, DialogResult};
use crate::ui::input::Field;
use crate::ui::{Ui, pane};

#[derive(Clone, Debug)]
pub struct FontsTab {
    entries: Vec<scan::FontEntry>,
    state: ListState,
    filter: Field,
    show_filter: bool,
    slot: usize,
    session: apply::SessionState,
    prompt: Vec<String>,
}

impl FontsTab {
    pub fn new(paths: &Paths) -> Self {
        // ensure builtin seed
        if scan::list_library(paths).unwrap_or_default().is_empty() {
            let slot_path = paths.font_slot_path(FontSlot::Regular);
            if slot_path.exists() {
                let _ = std::fs::create_dir_all(paths.fonts_dir());
                let dest = paths.fonts_dir().join("Current.ttf");
                let _ = std::fs::copy(&slot_path, &dest);
            }
        }
        let entries = scan::list_library(paths).unwrap_or_default();
        let mut state = ListState::default();
        if !entries.is_empty() {
            state.select(Some(0));
        }
        let prompt = prompt::capture_prompt_lines();
        Self {
            entries,
            state,
            filter: Field::new(""),
            show_filter: false,
            slot: 0,
            session: apply::new_session_state(paths),
            prompt,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, paths: &Paths) -> ScreenAction {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                if self.entries.is_empty() {
                    return ScreenAction::None;
                }
                let i = self.state.selected().unwrap_or(0);
                self.state.select(Some((i + 1).min(self.entries.len() - 1)));
                ScreenAction::None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.entries.is_empty() {
                    return ScreenAction::None;
                }
                let i = self.state.selected().unwrap_or(0);
                if i > 0 {
                    self.state.select(Some(i - 1));
                }
                ScreenAction::None
            }
            KeyCode::Char('s') => {
                self.slot = (self.slot + 1) % FontSlot::ALL.len();
                ScreenAction::Status(format!("Slot: {}", FontSlot::ALL[self.slot].as_str()))
            }
            KeyCode::Char('p') | KeyCode::Char(' ') => {
                if let Some(i) = self.state.selected() {
                    if let Some(entry) = self.entries.get(i) {
                        let _ = apply::preview_font(
                            paths,
                            &entry.path,
                            FontSlot::ALL[self.slot],
                            &mut self.session,
                        );
                        return ScreenAction::Status(format!("Previewing {}", entry.name));
                    }
                }
                ScreenAction::None
            }
            KeyCode::Enter => {
                if apply::is_dirty(&self.session) {
                    let _ = apply::commit_preview(paths, &mut self.session);
                    return ScreenAction::Status("Font applied".to_string());
                }
                ScreenAction::None
            }
            KeyCode::Esc => {
                if apply::is_dirty(&self.session) {
                    let _ = apply::restore_original(paths, &mut self.session);
                    return ScreenAction::Status("Restored original font".to_string());
                }
                ScreenAction::None
            }
            KeyCode::Char('i') => ScreenAction::OpenDialog(Dialog::message(
                "Import: use CLI or copy font files to ~/.termux/fonts/",
                "Import",
            )),
            KeyCode::Char('d') => {
                // trigger download of first nerd font as demo
                let catalog = crate::fonts::downloader::nerd_fonts_catalog();
                if let Some((name, _)) = catalog.iter().next() {
                    let name_clone = name.clone();
                    let paths_clone = paths.clone();
                    std::thread::spawn(move || {
                        let _ =
                            crate::fonts::downloader::fetch(&paths_clone, &name_clone, false, None);
                    });
                    ScreenAction::Status(format!("Downloading {name}..."))
                } else {
                    ScreenAction::None
                }
            }
            KeyCode::Char('/') => {
                self.show_filter = !self.show_filter;
                ScreenAction::None
            }
            _ => {
                if self.show_filter {
                    self.filter.handle_key(key);
                }
                ScreenAction::None
            }
        }
    }

    pub fn opened(&mut self, paths: &Paths) -> ScreenAction {
        self.entries = scan::list_library(paths).unwrap_or_default();
        ScreenAction::None
    }
    pub fn command(&mut self, _command: Command, _paths: &Paths) -> ScreenAction {
        ScreenAction::None
    }
    pub fn on_dialog(&mut self, _result: DialogResult, _paths: &Paths) -> ScreenAction {
        ScreenAction::None
    }

    /// Restore any uncommitted preview before the application quits.
    pub fn on_quit(&mut self, paths: &Paths) -> ScreenAction {
        if apply::is_dirty(&self.session) {
            let _ = apply::restore_original(paths, &mut self.session);
        }
        ScreenAction::None
    }

    /// Handle a worker message: a captured prompt or a fresh library scan.
    pub fn on_msg(&mut self, msg: &Msg, _paths: &Paths) -> ScreenAction {
        match msg {
            Msg::PromptCaptured(lines) => {
                self.prompt = lines.clone();
                ScreenAction::None
            }
            Msg::FontsScanned(Ok(entries)) => {
                self.entries = entries.clone();
                ScreenAction::None
            }
            _ => ScreenAction::None,
        }
    }

    /// The key bindings the help bar shows for this screen.
    pub fn help(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            ("↑/↓", "move"),
            ("p", "preview"),
            ("Enter", "apply"),
            ("s", "slot"),
            ("i", "import"),
            ("d", "download"),
            ("/", "filter"),
            ("Esc", "back"),
        ]
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, ui: &Ui) {
        frame.render_widget(
            Block::default().style(
                Style::default()
                    .bg(ui.palette.background)
                    .fg(ui.palette.foreground),
            ),
            area,
        );
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(2),
                Constraint::Min(5),
            ])
            .split(area);
        // preview area
        let preview: Vec<ratatui::text::Line> = self
            .prompt
            .iter()
            .map(|l| ratatui::text::Line::from(l.as_str()))
            .collect();
        frame.render_widget(
            Paragraph::new(preview).block(pane(ui, " Preview")),
            layout[0],
        );
        // slot info
        let (open, close) = ui.glyphs.chip();
        let slot = FontSlot::ALL[self.slot].as_str();
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("slot  ", Style::default().fg(ui.palette.muted)),
                Span::styled(
                    format!("{open} {slot} {close}"),
                    Style::default()
                        .fg(chrome::accent(ui.palette.dark))
                        .add_modifier(Modifier::BOLD),
                ),
            ])),
            layout[1],
        );
        // font list
        let items: Vec<ListItem> = self
            .entries
            .iter()
            .map(|e| ListItem::new(format!("{} - {} {}", e.name, e.family, e.style)))
            .collect();
        let title = format!(" Fonts ({})", self.entries.len());
        let list = List::new(items)
            .block(pane(ui, &title))
            .highlight_symbol(ui.glyphs.marker())
            .highlight_style(
                Style::default()
                    .bg(chrome::accent(ui.palette.dark))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_stateful_widget(list, layout[2], &mut self.state);
    }
}
