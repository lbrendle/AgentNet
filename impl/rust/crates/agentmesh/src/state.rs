use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub struct SequenceStore {
    path: PathBuf,
    current: u64,
}

impl SequenceStore {
    pub fn load(state_dir: &Path, name: &str) -> Result<Self> {
        fs::create_dir_all(state_dir).with_context(|| format!("create state dir {}", state_dir.display()))?;
        let path = state_dir.join(name);
        let current = match fs::read_to_string(&path) {
            Ok(data) => data.trim().parse::<u64>().unwrap_or(0),
            Err(_) => 0,
        };
        Ok(Self { path, current })
    }

    pub fn next(&mut self) -> Result<u64> {
        self.current = self.current.saturating_add(1);
        fs::write(&self.path, format!("{}\n", self.current))
            .with_context(|| format!("write seq {}", self.path.display()))?;
        Ok(self.current)
    }
}
