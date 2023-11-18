use ansi_to_tui::IntoText;
use ratatui::{prelude::*, widgets::*};
use std::io::{BufRead, BufReader, Error, ErrorKind, Result};
use std::process::{Child, Command, Stdio};
use std::str::FromStr;
use std::sync::mpsc;
use std::thread;

pub struct Options {
    pub show_hidden: bool,
    pub prompt: String,
    pub glob: String,
}

pub struct Job<'a> {
    process: Child,
    rx: mpsc::Receiver<Vec<u8>>,

    results_items: Vec<ListItem<'a>>,
    results_files: Vec<Option<String>>,
    results_lines: Vec<Option<i32>>,
}

impl<'a> Job<'a> {
    pub fn new(options: &Options) -> Result<Self> {
        let mut command = Self::build_command(options);
        command.stderr(Stdio::null());
        let mut process = command.stdout(Stdio::piped()).spawn()?;
        let Some(stdout) = process.stdout.take() else {
            return Err(Error::new(ErrorKind::Other, "No stdout"));
        };

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);

            loop {
                let mut line: Vec<u8> = Vec::new();
                let num_bytes = reader.read_until(b'\n', &mut line).unwrap_or(0);
                if num_bytes == 0 {
                    break;
                }
                if let Err(_) = tx.send(line) {
                    break;
                }
            }
        });

        Ok(Job {
            process,
            rx,

            results_items: Vec::new(),
            results_files: Vec::new(),
            results_lines: Vec::new(),
        })
    }

    pub fn get_results_items(&self) -> &[ListItem] {
        &self.results_items[..]
    }

    pub fn get_result(&self, index: usize) -> (Option<&str>, Option<i32>) {
        if index >= self.current_num_results() {
            return (None, None);
        }

        return (
            self.results_files[index].as_deref(),
            self.results_lines[index],
        );
    }

    pub fn current_num_results(&self) -> usize {
        self.results_items.len()
    }

    pub fn try_read_next_result(&mut self) -> Result<bool> {
        match self.rx.try_recv() {
            Ok(line) => {
                self.read_next_result(line)?;
                Ok(true)
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.finalize()?;
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    fn read_next_result(&mut self, line: Vec<u8>) -> Result<()> {
        let text = match line.into_text() {
            Ok(t) => t,
            Err(_) => Text::from("Error"),
        };

        let mut line_iter = line.iter();
        let first_sep = line_iter.position(|&c| c == b':');
        let second_sep = line_iter.position(|&c| c == b':');

        let file_name = match first_sep {
            Some(first) => Self::parse_bytes(&line[..first]),
            _ => None,
        };

        let line_number = match (first_sep, second_sep) {
            (Some(first), Some(second)) => Self::parse_bytes(&line[first + 1..first + 1 + second]),
            _ => None,
        };

        self.results_items.push(ListItem::new(text));
        self.results_files.push(file_name);
        self.results_lines.push(line_number);

        Ok(())
    }

    fn parse_bytes<T: FromStr>(s: &[u8]) -> Option<T> {
        let Ok(string) = String::from_utf8(strip_ansi_escapes::strip(s)) else {
            return None;
        };

        match string.parse() {
            Ok(res) => Some(res),
            _ => None,
        }
    }

    pub fn finalize(&mut self) -> Result<()> {
        self.process.kill()?;
        // TODO: don't wait here?
        self.process.wait()?;
        Ok(())
    }

    fn build_command(options: &Options) -> Command {
        let mut command = Command::new("rg");
        command
            .arg("--column")
            .arg("--color=always")
            .arg(if options.show_hidden {
                "--hidden"
            } else {
                "--no-hidden"
            });

        for glob in options.glob.split(";") {
            command.arg("--glob").arg(glob.trim());
        }
        command.arg(&options.prompt);
        command
    }
}
