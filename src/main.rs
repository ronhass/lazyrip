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
    show_glob: bool,

    prompt: TextArea<'a>,
    glob: TextArea<'a>,

    results_manager: results::Manager<'a>,
}

impl<'a> App<'a> {
    fn new() -> App<'a> {
        let mut textarea = TextArea::default();
        textarea.set_placeholder_text("Start typing to search...");
        textarea.set_block(Self::default_block());
        textarea.set_cursor_line_style(Style::default());

        let mut glob_textarea = TextArea::default();
        glob_textarea.set_placeholder_text("Empty");
        glob_textarea.set_block(
            Self::default_block()
                .title(" Glob (use ; to separate multiple) ")
                .title_alignment(Alignment::Center),
        );
        glob_textarea.set_cursor_line_style(Style::default());

        App {
            prompt: textarea,
            glob: glob_textarea,
            should_quit: false,
            should_restart_terminal: false,
            show_glob: false,
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
        loop {
            let mut should_rerender = false;

            if event::poll(std::time::Duration::from_millis(20))? {
                should_rerender = true;
                if self.show_glob {
                    self.glob_mode()?;
                } else {
                    self.main_mode()?;
                }
            }

            should_rerender = self.results_manager.update()? || should_rerender;

            if should_rerender {
                break;
            }
        }
        Ok(())
    }

    fn main_mode(&mut self) -> io::Result<()> {
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
            } => self.results_manager.toggle_preview(),
            Input {
                key: Key::Char('g'),
                ctrl: true,
                ..
            } => self.show_glob = true,
            Input { key: Key::Down, .. } => self.results_manager.next(),
            Input { key: Key::Up, .. } => self.results_manager.prev(),
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

        Ok(())
    }

    fn glob_mode(&mut self) -> io::Result<()> {
        match event::read()?.into() {
            Input { key: Key::Esc, .. }
            | Input {
                key: Key::Char('c'),
                ctrl: true,
                ..
            }
            | Input {
                key: Key::Char('g'),
                ctrl: true,
                ..
            }
            | Input {
                key: Key::Enter, ..
            }
            | Input {
                key: Key::Char('m'),
                ctrl: true,
                ..
            } => self.show_glob = false,
            input => {
                if self.glob.input(input) {
                    self.results_manager.set_glob(self.glob.lines()[0].clone())
                }
            }
        };

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

        let results_layout = if self.results_manager.show_preview {
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
            Span::styled("<C+g>", Style::default().fg(Color::Red)),
            Span::raw(": Edit glob "),
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

        if self.show_glob {
            let popup_area = App::centered_rect(50, 5, frame.size());
            frame.render_widget(Clear, popup_area);
            frame.render_widget(self.glob.widget(), popup_area);
        }
    }

    /// helper function from ratatui
    fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }
}

fn main() -> io::Result<()> {
    App::new().run()?;
    Ok(())
}
