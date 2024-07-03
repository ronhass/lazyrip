use ansi_to_tui::IntoText;
use ratatui::{prelude::*, widgets::*};
use std::io::{BufRead, BufReader, Error, ErrorKind, Result};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;

pub struct Preview {
    text: Text<'static>,
    line_number: i32,
}

impl Preview {
    pub fn new(text: Text<'static>, line_number: i32) -> Preview {
        Preview { text, line_number }
    }

    pub fn get_paragraph(&self, height: i32) -> Paragraph {
        let paragraph = Paragraph::new(self.text.clone());
        let scroll_y: u16 = std::cmp::max(0, self.line_number - height / 2)
            .try_into()
            .unwrap_or(0);
        paragraph.scroll((scroll_y, 0))
    }
}

pub struct PreviewJob {
    process: Child,
    line_number: i32,
    rx: mpsc::Receiver<Vec<u8>>,
    content: Vec<u8>,
}

impl PreviewJob {
    pub fn new(file_path: &str, line_number: i32) -> Result<Self> {
        let mut command = Self::build_command(file_path, line_number);
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

        Ok(PreviewJob {
            process,
            line_number,
            rx,
            content: Vec::new(),
        })
    }

    pub fn try_recv_preview(&mut self) -> Result<(Preview, bool)> {
        match self.rx.try_recv() {
            Ok(line) => {
                self.content.extend(line);
                Ok((Preview::new(Text::raw("Loading..."), 0), false))
            }
            Err(mpsc::TryRecvError::Empty) => Ok((Preview::new(Text::raw("Loading..."), 0), false)),
            Err(mpsc::TryRecvError::Disconnected) => {
                self.finalize()?;
                match self.content.into_text() {
                    Ok(text) => Ok((Preview::new(text, self.line_number), true)),
                    _ => Err(Error::new(ErrorKind::Other, "couldn't read preview")),
                }
            }
        }
    }

    pub fn finalize(&mut self) -> Result<()> {
        self.process.kill()?;
        // TODO: don't wait here?
        self.process.wait()?;
        Ok(())
    }

    fn build_command(file_path: &str, line_number: i32) -> Command {
        let mut command = Command::new("bat");
        command
            .arg("--color=always")
            .arg("-n")
            .arg("-H")
            .arg(line_number.to_string())
            .arg(file_path);
        command
    }
}
