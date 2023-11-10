mod results;

use crossterm::{
    event,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};
use std::io::{self, stdout};
use tui_textarea::{Input, Key, TextArea};

struct App<'a> {
    should_quit: bool,
    should_restart_terminal: bool,

    prompt: TextArea<'a>,

    preview: bool,

    results_manager: results::Manager<'a>,
}

impl<'a> App<'a> {
    fn new() -> App<'a> {
        let mut textarea = TextArea::default();
        textarea.set_placeholder_text("Start typing to search...");
        textarea.set_block(Self::default_block());
        textarea.set_cursor_line_style(Style::default());

        App {
            prompt: textarea,
            should_quit: false,
            should_restart_terminal: false,
            preview: true,
            results_manager: results::Manager::new(),
        }
    }

    fn default_block() -> Block<'static> {
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
    }

    fn startup(&mut self) -> io::Result<()> {
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;
        Ok(())
    }

    fn shutdown(&self) -> io::Result<()> {
        disable_raw_mode()?;
        stdout().execute(LeaveAlternateScreen)?;
        Ok(())
    }

    fn run(&mut self) -> io::Result<()> {
        loop {
            self.should_restart_terminal = false;
            self.startup()?;
            let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

            while !self.should_restart_terminal {
                terminal.draw(|f| {
                    self.ui(f);
                })?;
                match self.handle_events() {
                    Ok(_) => (),
                    e => {
                        let _ = self.shutdown();
                        return e;
                    }
                }

                if self.should_quit {
                    self.shutdown()?;
                    return Ok(());
                }
            }

            self.shutdown()?;
        }
    }

    fn handle_events(&mut self) -> io::Result<()> {
        if event::poll(std::time::Duration::from_millis(50))? {
            match event::read()?.into() {
                Input {
                    key: Key::Char('c'),
                    ctrl: true,
                    ..
                } => self.should_quit = true,
                Input {
                    key: Key::Char('h'),
                    ctrl: true,
                    ..
                } => self.results_manager.toggle_hidden(),
                Input {
                    key: Key::Char('p'),
                    ctrl: true,
                    ..
                } => self.preview = !self.preview,
                Input { key: Key::Down, .. } => self.results_manager.next()?,
                Input { key: Key::Up, .. } => self.results_manager.prev()?,
                Input { key: Key::Esc, .. } => (),
                Input {
                    key: Key::Enter, ..
                }
                | Input {
                    key: Key::Char('m'),
                    ctrl: true,
                    ..
                } => self.should_restart_terminal = self.results_manager.open_selection(),
                input => {
                    if self.prompt.input(input) {
                        self.results_manager
                            .set_prompt(self.prompt.lines()[0].clone())
                    }
                }
            };

            self.results_manager.execute()?;
        }
        Ok(())
    }

    fn ui(&mut self, frame: &mut Frame) {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(frame.size());

        let top_line = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1), Constraint::Length(20)])
            .split(main_layout[0]);

        frame.render_widget(self.prompt.widget(), top_line[0]);
        let s = if self.results_manager.is_showing_hidden() {
            "🗹 Show hidden"
        } else {
            "☐ Show hidden"
        };
        frame.render_widget(Paragraph::new(s).block(Self::default_block()), top_line[1]);

        let results_layout = if self.preview {
            let body = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main_layout[1]);

            frame.render_widget(
                self.results_manager
                    .get_preview(body[0].height.into())
                    .block(Self::default_block().title(" Preview ")),
                body[1],
            );

            body[0]
        } else {
            main_layout[1]
        };
        frame.render_stateful_widget(
            self.results_manager
                .get_list()
                .block(Self::default_block().title(" Results "))
                .highlight_symbol("»"),
            results_layout,
            &mut self.results_manager.get_list_state(),
        );

        let line = Line::from(vec![
            Span::styled("↑↓", Style::default().fg(Color::Red)),
            Span::raw(": Navigate results "),
            Span::styled("ENTER", Style::default().fg(Color::Red)),
            Span::raw(": Open file "),
            Span::styled("<C+p>", Style::default().fg(Color::Red)),
            Span::raw(": Toggle preview "),
            Span::styled("<C+h>", Style::default().fg(Color::Red)),
            Span::raw(": Toggle search in hidden files "),
            Span::styled("<C+c>", Style::default().fg(Color::Red)),
            Span::raw(": Quit "),
        ]);
        frame.render_widget(
            Paragraph::new(Text::from(line)).block(Self::default_block()),
            main_layout[2],
        );
    }
}

fn main() -> io::Result<()> {
    App::new().run()?;
    Ok(())
}
