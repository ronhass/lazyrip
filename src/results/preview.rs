use ansi_to_tui::IntoText;
use ratatui::{prelude::*, widgets::*};
use std::io::{Error, ErrorKind, Result};
use std::process::Command;

pub struct Preview {
    text: Text<'static>,
    line_number: Option<i32>,
}

impl Preview {
    pub fn new(file_path: &str, line_number: Option<i32>) -> Option<Preview> {
        match Self::preview_file(file_path, line_number) {
            Ok(text) => Some(Preview { text, line_number }),
            _ => None,
        }
    }

    pub fn get_paragraph(&self, height: i32) -> Paragraph {
        let mut paragraph = Paragraph::new(self.text.clone());
        if let Some(n) = self.line_number {
            let scroll_y: u16 = std::cmp::max(0, n - height / 2).try_into().unwrap_or(0);
            paragraph = paragraph.scroll((scroll_y, 0));
        }
        paragraph
    }

    fn preview_file(file_path: &str, line_number: Option<i32>) -> Result<Text<'static>> {
        let mut command = Command::new("bat");
        command.arg("--color=always").arg("-n");
        if let Some(n) = line_number {
            command.arg("-H").arg(n.to_string());
        }
        command.arg(file_path);

        let output = command.output()?.stdout;
        match output.into_text() {
            Ok(t) => Ok(t),
            _ => Err(Error::new(ErrorKind::Other, "")),
        }
    }
}
