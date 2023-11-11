mod preview;
mod ripgrep;

use ratatui::widgets::*;
use std::io::Result;
use std::process::Command;

pub struct Manager<'a> {
    should_execute: bool,
    job: Option<ripgrep::Job<'a>>,
    pub show_preview: bool,

    options: ripgrep::Options,

    selection_index: Option<usize>,
    selection_preview: Option<preview::Preview>,
}

impl<'a> Manager<'a> {
    pub fn new() -> Manager<'a> {
        return Manager {
            should_execute: false,
            job: None,
            show_preview: true,

            options: ripgrep::Options {
                show_hidden: false,
                prompt: String::new(),
                glob: String::new(),
            },

            selection_index: None,
            selection_preview: None,
        };
    }

    pub fn set_prompt(&mut self, prompt: String) {
        self.options.prompt = prompt;
        self.should_execute = true;
    }

    pub fn set_glob(&mut self, glob: String) {
        self.options.glob = glob;
        self.should_execute = true;
    }

    pub fn toggle_hidden(&mut self) {
        self.options.show_hidden = !self.options.show_hidden;
        self.should_execute = true;
    }

    pub fn is_showing_hidden(&self) -> bool {
        self.options.show_hidden
    }

    pub fn next(&mut self) -> Result<()> {
        let Some(index) = self.selection_index else {
            return Ok(());
        };
        let Some(job) = self.job.as_ref() else {
            return Ok(());
        };

        let num_results = job.current_num_results();
        self.select(Some(if index + 1 == num_results {
            index
        } else {
            index + 1
        }));

        self.read_results_if_necessary()
    }

    pub fn prev(&mut self) -> Result<()> {
        let Some(index) = self.selection_index else {
            return Ok(());
        };

        self.select(Some(if index == 0 { index } else { index - 1 }));
        Ok(())
    }

    fn select(&mut self, selection: Option<usize>) {
        self.selection_index = selection;
        self.update_preview();
    }

    pub fn toggle_preview(&mut self) {
        self.show_preview = !self.show_preview;
        self.update_preview();
    }

    fn update_preview(&mut self) {
        self.selection_preview = None;

        if !self.show_preview {
            return;
        }

        let Some(index) = self.selection_index else {
            return;
        };
        let Some(job) = self.job.as_ref() else {
            return;
        };

        let (Some(file_path), line_number) = job.get_result(index) else {
            return;
        };

        self.selection_preview = preview::Preview::new(file_path, line_number);
    }

    pub fn execute(&mut self) -> Result<()> {
        if !self.should_execute {
            return Ok(());
        }
        self.should_execute = false;

        if let Some(mut j) = self.job.take() {
            j.finalize()?;
        }

        self.select(None);

        if self.options.prompt.len() == 0 {
            return Ok(());
        }

        self.job = Some(ripgrep::Job::new(&self.options)?);
        self.read_results_if_necessary()?;

        Ok(())
    }

    fn read_results_if_necessary(&mut self) -> Result<()> {
        let Some(job) = self.job.as_mut() else {
            return Ok(());
        };

        let result_number_to_reach = match self.selection_index {
            None => 100,
            Some(i) => 100 + i,
        };

        while job.current_num_results() < result_number_to_reach {
            if !job.read_next_result()? {
                break;
            }
        }

        if self.selection_index == None && job.current_num_results() > 0 {
            self.select(Some(0));
        }

        Ok(())
    }

    pub fn get_list(&self) -> List {
        match self.job.as_ref() {
            None => List::new(vec![]),
            Some(job) => List::new(job.get_results_items()),
        }
    }

    pub fn get_list_state(&self) -> ListState {
        ListState::default().with_selected(self.selection_index)
    }

    pub fn get_preview(&self, height: i32) -> Paragraph {
        match &self.selection_preview {
            Some(t) => t.get_paragraph(height),
            None => Paragraph::new(""),
        }
    }

    pub fn open_selection(&self) -> bool {
        let Some(index) = self.selection_index else {
            return false;
        };
        let Some(job) = self.job.as_ref() else {
            return false;
        };

        let (Some(file_path), line_number) = job.get_result(index) else {
            return false;
        };

        let command = match line_number {
            None => format!("$EDITOR \"{}\"", file_path),
            Some(n) => format!("$EDITOR +{} \"{}\"", n, file_path),
        };

        let _ = Command::new("sh").arg("-c").arg(command).status();
        true
    }
}
