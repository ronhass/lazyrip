use ansi_to_tui::IntoText;
use crossterm::{
    event,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};
use std::io::{self, stdout};
use std::process::Command;
use tui_textarea::{Input, Key, TextArea};

struct App<'a> {
    should_quit: bool,
    should_restart: bool,

    prompt: TextArea<'a>,

    display_hidden: bool,

    raw_result: String,
    result_lines: Vec<String>,
    result_items: Vec<ListItem<'a>>,
    result_state: ListState,
    result_index: Option<usize>,
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
            should_restart: false,
            display_hidden: false,
            raw_result: String::from(""),
            result_lines: vec![],
            result_items: vec![],
            result_state: ListState::default(),
            result_index: None,
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
            self.should_restart = false;
            self.startup()?;
            let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

            while !self.should_restart {
                terminal.draw(|f| {
                    self.ui(f);
                })?;
                self.handle_events()?;

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
            let should_recalc = match event::read()?.into() {
                Input {
                    key: Key::Char('c'),
                    ctrl: true,
                    ..
                } => {
                    self.should_quit = true;
                    false
                }
                Input {
                    key: Key::Char('h'),
                    ctrl: true,
                    ..
                } => {
                    self.display_hidden = !self.display_hidden;
                    true
                }
                Input { key: Key::Down, .. } => {
                    self.result_index = match self.result_index {
                        None => None,
                        Some(i) => Some((i + 1) % self.result_lines.len()),
                    };
                    self.result_state.select(self.result_index);
                    false
                }
                Input { key: Key::Up, .. } => {
                    self.result_index = match self.result_index {
                        None => None,
                        Some(i) => {
                            Some((i + self.result_lines.len() - 1) % self.result_lines.len())
                        }
                    };
                    self.result_state.select(self.result_index);
                    false
                }
                Input { key: Key::Esc, .. } => false,
                Input {
                    key: Key::Enter, ..
                }
                | Input {
                    key: Key::Char('m'),
                    ctrl: true,
                    ..
                } => {
                    if let Some(i) = self.result_index {
                        let result_line = strip_ansi_escapes::strip_str(&self.result_lines[i]);
                        let mut splitted = result_line.split(":");
                        let file = splitted.next().unwrap();
                        let lineno = splitted.next().unwrap();

                        self.should_restart = true;
                        let _ = Command::new("nvim")
                            .arg(format!("+{}", lineno))
                            .arg(file)
                            .status();
                    }
                    false
                }
                input => self.prompt.input(input),
            };

            if should_recalc {
                self.execute_rg();
            }
        }
        Ok(())
    }

    fn ui(&mut self, frame: &mut Frame) {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(frame.size());

        let top_line = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1), Constraint::Length(20)])
            .split(main_layout[0]);

        frame.render_widget(self.prompt.widget(), top_line[0]);
        let s = if self.display_hidden {
            "🗹 Show hidden"
        } else {
            "☐ Show hidden"
        };
        frame.render_widget(Paragraph::new(s).block(Self::default_block()), top_line[1]);
        frame.render_stateful_widget(
            List::new(&*self.result_items)
                .block(Self::default_block())
                .highlight_symbol("»"),
            main_layout[1],
            &mut self.result_state,
        );
    }

    fn execute_rg(&mut self) {
        self.result_lines = vec![];
        self.result_items = vec![];
        self.raw_result = String::from("");
        self.result_index = None;
        self.result_state.select(self.result_index);

        let prompt_str = &self.prompt.lines()[0];
        if prompt_str.len() == 0 {
            return;
        }

        // TODO: I copy everything here because I don't know how to work with lifetimes
        let output = self.build_rg_command(prompt_str).output().unwrap().stdout;
        self.raw_result = String::from_utf8(output).unwrap();
        self.result_lines = self
            .raw_result
            .split("\n")
            .filter(|s| s.len() > 0)
            .map(|s| s.to_string())
            .collect();
        self.result_items = self
            .result_lines
            .iter()
            .map(|l| ListItem::new(l.into_text().unwrap()))
            .collect();
        if self.result_lines.len() > 0 {
            self.result_index = Some(0);
            self.result_state.select(self.result_index);
        }
    }

    fn build_rg_command(&self, prompt_str: &str) -> Command {
        let mut command = Command::new("rg");
        command.arg("--column");
        command.arg("--color=always");
        if self.display_hidden {
            command.arg("--hidden");
        } else {
            command.arg("--no-hidden");
        }
        command.arg(prompt_str);
        command
    }
}

fn main() -> io::Result<()> {
    App::new().run()?;
    Ok(())
}
