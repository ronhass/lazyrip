use ansi_to_tui::IntoText;
use ratatui::{prelude::*, widgets::*};
use std::io::{Error, ErrorKind, Read, Result};
use std::process::{Command, Stdio};
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
    line_number: i32,
    rx: mpsc::Receiver<Result<Vec<u8>>>,
}

impl PreviewJob {
    pub fn new(file_path: &str, line_number: i32) -> Result<Self> {
        let mut command = Self::build_command(file_path, line_number);
        command.stderr(Stdio::null());
        let mut process = command.stdout(Stdio::piped()).spawn()?;
        let Some(mut stdout) = process.stdout.take() else {
            return Err(Error::new(ErrorKind::Other, "No stdout"));
        };
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let mut buffer: Vec<u8> = Vec::new();
            match stdout.read_to_end(&mut buffer) {
                Ok(_) => tx.send(Ok(buffer)),
                Err(e) => tx.send(Err(e)),
            }
        });

        Ok(PreviewJob { line_number, rx })
    }

    pub fn try_recv_preview(&self) -> Result<Option<Preview>> {
        match self.rx.try_recv() {
            Ok(maybe_output) => {
                let output = maybe_output?;
                match output.into_text() {
                    Ok(text) => Ok(Some(Preview::new(text, self.line_number))),
                    _ => Err(Error::new(ErrorKind::Other, "Could not parse output")),
                }
            }
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => {
                Err(Error::new(ErrorKind::Other, "Thread Disconnected"))
            }
        }
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
