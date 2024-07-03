use ansi_to_tui::IntoText;
use ratatui::{prelude::*, widgets::*};
use std::process::Command;
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
    rx: mpsc::Receiver<Option<Vec<u8>>>,
}

impl PreviewJob {
    pub fn new(file_path: &str, line_number: i32) -> PreviewJob {
        let mut command = Self::build_command(file_path, line_number);
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || match command.output() {
            Ok(output) => tx.send(Some(output.stdout)),
            _ => tx.send(None),
        });

        PreviewJob { line_number, rx }
    }

    pub fn try_recv_preview(&self) -> Option<Preview> {
        let Ok(maybe_output) = self.rx.recv() else {
            return None;
        };

        match maybe_output {
            Some(output) => match output.into_text() {
                Ok(text) => Some(Preview::new(text, self.line_number)),
                _ => Some(Preview::new(Text::raw("Error in parsing preview"), 0)),
            },
            None => Some(Preview::new(Text::raw("Error in bat"), 0)),
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
