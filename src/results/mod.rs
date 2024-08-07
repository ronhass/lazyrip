mod preview;
mod ripgrep;

use ratatui::widgets::*;
use std::io::Result;
use std::process::Command;

pub struct Manager<'a> {
    should_execute: bool,
    should_rerender: bool,
    job: Option<ripgrep::Job<'a>>,
    preview_job: Option<preview::PreviewJob>,
    pub show_preview: bool,

    options: ripgrep::Options,

    selection_index: Option<usize>,
    selection_preview: Option<preview::Preview>,
}

impl<'a> Manager<'a> {
    pub fn new() -> Manager<'a> {
        return Manager {
            should_execute: false,
            should_rerender: true,
            job: None,
            preview_job: None,
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
        let Some(job) = self.job.as_ref() else {
            return Ok(());
        };
        let num_results = job.current_num_results();

        match self.selection_index {
            None => {
                if num_results > 0 {
                    self.select(Some(0))?;
                }
            }
            Some(index) => {
                if index + 1 < num_results {
                    self.select(Some(index + 1))?;
                }
            }
        };

        Ok(())
    }

    pub fn prev(&mut self) -> Result<()> {
        let Some(index) = self.selection_index else {
            return Ok(());
        };
        if index > 0 {
            self.select(Some(index - 1))?;
        }

        Ok(())
    }

    fn select(&mut self, selection: Option<usize>) -> Result<()> {
        self.selection_index = selection;
        self.should_rerender = true;
        self.update_preview()
    }

    pub fn toggle_preview(&mut self) -> Result<()> {
        self.show_preview = !self.show_preview;
        self.should_rerender = true;
        self.update_preview()
    }

    fn update_preview(&mut self) -> Result<()> {
        self.selection_preview = None;

        if !self.show_preview {
            return Ok(());
        }

        let Some(index) = self.selection_index else {
            return Ok(());
        };
        let Some(job) = self.job.as_ref() else {
            return Ok(());
        };

        let (file_path, line_number) = job.get_result(index);
        self.preview_job = Some(preview::PreviewJob::new(file_path, line_number)?);
        Ok(())
    }

    pub fn update(&mut self) -> Result<bool> {
        if self.should_execute {
            self.execute_job()
        } else {
            self.read_jobs()
        }
    }

    fn execute_job(&mut self) -> Result<bool> {
        self.select(None)?;

        if let Some(mut j) = self.job.take() {
            j.finalize()?;
        }

        if self.options.prompt.len() > 0 {
            self.job = Some(ripgrep::Job::new(&self.options)?);
        }

        self.should_execute = false;
        self.should_rerender = false;
        Ok(true)
    }

    fn read_jobs(&mut self) -> Result<bool> {
        let mut should_rerender = self.should_rerender;

        if let Some(j) = self.job.as_mut() {
            for _ in 1..10 {
                if j.try_read_next_result()? {
                    should_rerender = true;
                } else {
                    break;
                }
            }
        }

        if let Some(preview_job) = self.preview_job.as_ref() {
            if let Some(preview) = preview_job.try_recv_preview()? {
                self.selection_preview = Some(preview);
                self.preview_job = None;
                should_rerender = true;
            }
        }

        self.should_rerender = false;
        Ok(should_rerender)
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

        let (file_path, line_number) = job.get_result(index);
        let command = format!("$EDITOR +{} \"{}\"", line_number, file_path);
        let _ = Command::new("sh").arg("-c").arg(command).status();
        true
    }
}
