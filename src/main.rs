use std::io::{self, stdout};
use std::process::Command;
use crossterm::{
    event,
    ExecutableCommand,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}
};
use ratatui::{prelude::*, widgets::*};
use tui_textarea::{Input, Key, TextArea};
use ansi_to_tui::IntoText;

struct App<'a> {
    prompt: TextArea<'a>,
    should_quit: bool,
    raw_results: Vec<u8>,
    hidden: bool,
}

impl<'a> App<'a> {
    fn new() -> io::Result<App<'a>> {
        let mut textarea = TextArea::default();
        textarea.set_placeholder_text("Type here...");
        textarea.set_block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded));
        textarea.set_cursor_line_style(Style::default());

        Ok(App{
            prompt: textarea,
            should_quit: false,
            raw_results: vec!{},
            hidden: false,
        })
    }

    fn run(&mut self) -> io::Result<()> {
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

        while !self.should_quit {
            terminal.draw(|f| {
                self.ui(f);
            })?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn handle_events(&mut self) -> io::Result<()> {
        if event::poll(std::time::Duration::from_millis(50))? {
            let prompt_changed = match event::read()?.into() {
                Input { key: Key::Char('c'), ctrl: true, .. } => {
                    self.should_quit = true;
                    false
                },
                Input { key: Key::Char('h'), ctrl: true, .. } => {
                    self.hidden = !self.hidden;
                    true
                }
                Input { key: Key::Esc, .. } => false,
                Input { key: Key::Enter, .. } | Input { key: Key::Char('m'), ctrl: true, .. } => false,
                input => self.prompt.input(input),
            };

            if prompt_changed {
                let output = self.build_rg_command().arg(&self.prompt.lines()[0]).output().expect("Fail");
                self.raw_results = output.stdout;
            }
        }
        Ok(())
    }

    fn ui(&mut self, frame: &mut Frame) {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                         Constraint::Length(3),
                         Constraint::Min(1),
            ])
            .split(frame.size());
        frame.render_widget(
            self.prompt.widget(),
            main_layout[0],
        );
        frame.render_widget(
            Paragraph::new(self.raw_results.into_text().unwrap()).block(Block::new().borders(Borders::ALL).border_type(BorderType::Rounded)),
            main_layout[1],
        );
    }

    fn build_rg_command(&self) -> Command {
        let mut command = Command::new("rg");
        command.arg("--column");
        command.arg("--color=always");
        if self.hidden {
            command.arg("--hidden");
        } else {
            command.arg("--no-hidden");
        }
        command
    }
}

fn startup() -> io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    Ok(())
}

fn shutdown() -> io::Result<()> {
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn main() -> io::Result<()> {
    startup()?;

    let mut app = App::new()?;
    app.run()?;

    shutdown()?;
    Ok(())
}
