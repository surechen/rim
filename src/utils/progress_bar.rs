//! Progress bar indicator for commandline user interface.

use std::sync::{Arc, Mutex};

use anyhow::Result;
use indicatif::{ProgressBar as CliProgressBar, ProgressState, ProgressStyle};

struct ProgressPos(Mutex<f32>);

impl ProgressPos {
    fn new(value: f32) -> Self {
        Self(Mutex::new(value))
    }
    fn load(&self) -> f32 {
        *self.0.lock().unwrap()
    }
    /// Increment position value, and ensure the end result not exceeding 100.
    fn add(&self, value: f32) {
        let mut guard = self.0.lock().unwrap();
        *guard = (*guard + value).min(100.0);
    }
}

#[derive(Clone)]
pub struct Progress<'a> {
    pos: Arc<ProgressPos>,
    pub len: f32,
    msg_callback: &'a dyn Fn(String) -> Result<()>,
    pos_callback: &'a dyn Fn(f32) -> Result<()>,
}

impl<'a> Progress<'a> {
    pub fn new<M, P>(msg_cb: &'a M, pos_cb: &'a P) -> Self
    where
        M: Fn(String) -> Result<()>,
        P: Fn(f32) -> Result<()>,
    {
        Self {
            pos: Arc::new(ProgressPos::new(0.0)),
            len: 0.0,
            msg_callback: msg_cb,
            pos_callback: pos_cb,
        }
    }

    pub fn with_len(mut self, len: f32) -> Self {
        self.len = len;
        self
    }

    pub fn show_msg<S: ToString>(&self, msg: S) -> Result<()> {
        (self.msg_callback)(msg.to_string())
    }

    /// Update the position of progress bar by increment a certain value.
    ///
    /// If a value given is `None`, this will increase the position by the whole `len`,
    /// otherwise it will increase the desired value instead.
    // FIXME: split `inc(None)` to a new function, such as `inc_len`, cuz this is kinda confusing.
    pub fn inc(&self, value: Option<f32>) -> Result<()> {
        let delta = value.unwrap_or(self.len);
        self.pos.add(delta);
        (self.pos_callback)(self.pos.load())?;
        Ok(())
    }
}

/// Send the message via [`Progress`] and print it on console as well.
pub fn send_and_print<T: ToString>(msg: T, progress: Option<&Progress<'_>>) -> Result<()> {
    let m = msg.to_string();
    println!("{m}");
    if let Some(prog) = progress {
        prog.show_msg(m)?;
    }
    Ok(())
}

/// Convinent struct with methods that are useful to indicate various progress.
#[derive(Debug, Clone, Copy)]
pub struct CliProgress<T: Sized> {
    /// A start/initializing function which will be called to setup progress bar.
    pub start: fn(u64, String, Style) -> Result<T>,
    /// A update function that will be called upon each step completion.
    pub update: fn(&T, u64),
    /// A function that will be called once to terminate progress.
    pub stop: fn(&T, String),
}

#[derive(Debug, Default, Clone, Copy)]
pub enum Style {
    /// Display the progress base on number of bytes.
    Bytes,
    #[default]
    /// Display the progress base on position & length parameters.
    Len,
}

impl Style {
    fn template_str(&self) -> &str {
        match self {
            Style::Bytes => "{bytes}/{total_bytes}",
            Style::Len => "{pos}/{len}",
        }
    }
}

// TODO: Mark this with cfg(feature = "cli")
impl CliProgress<CliProgressBar> {
    /// Create a new progress bar for CLI to indicate download progress.
    ///
    /// `progress_for`: used for displaying what the progress is for.
    /// i.e.: ("downloading", "download"), ("extracting", "extraction"), etc.
    pub fn new() -> Self {
        fn start(total: u64, msg: String, style: Style) -> Result<CliProgressBar> {
            let pb = CliProgressBar::new(total);
            pb.set_style(
                ProgressStyle::with_template(
                    &format!("{{msg}}\n{{spinner:.green}}] [{{elapsed_precise}}] [{{wide_bar:.cyan/blue}}] {} ({{eta}})", style.template_str())
                )?
                .with_key("eta", |state: &ProgressState, w: &mut dyn std::fmt::Write| {
                    write!(w, "{:.1}s", state.eta().as_secs_f64()).expect("unable to display progress bar")
                })
                .progress_chars("#>-")
            );
            pb.set_message(msg);
            Ok(pb)
        }
        fn update(pb: &CliProgressBar, pos: u64) {
            pb.set_position(pos);
        }
        fn stop(pb: &CliProgressBar, msg: String) {
            pb.finish_with_message(msg);
        }

        CliProgress {
            start,
            update,
            stop,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ProgressPos;

    #[test]
    fn progress_pos_add() {
        let orig = ProgressPos::new(0.0);

        orig.add(1.0);
        assert_eq!(orig.load(), 1.0);
        orig.add(2.0);
        assert_eq!(orig.load(), 3.0);
        orig.add(10.0);
        assert_eq!(orig.load(), 13.0);
    }
}
