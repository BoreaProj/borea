use borea_core::*;
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::Path,
};
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub schema: u16,
    pub plan: String,
    pub config: String,
    pub matrix: String,
    pub sequence: u64,
    pub step: String,
    pub event: String,
    pub detail: Option<String>,
}
pub struct Journal {
    file: File,
    plan: String,
    config: String,
    matrix: String,
    sequence: u64,
}
impl Journal {
    pub fn open(path: &Path, plan: &Plan, resume: bool) -> Result<Self> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(!resume)
            .create(false)
            .mode(0o600)
            .open(path)?;
        file.try_lock()
            .map_err(|_| BoreaError::new("E_JOURNAL_LOCK", "journal is already in use"))?;
        let mut sequence = 0;
        if resume {
            use std::io::Read;
            let mut text = String::new();
            file.read_to_string(&mut text)?;
            if !text.is_empty() && !text.ends_with('\n') {
                return Err(BoreaError::new(
                    "E_JOURNAL",
                    "truncated journal; preserve it and investigate before resuming",
                ));
            }
            for line in text.lines() {
                let r: Record = serde_json::from_str(line)?;
                if r.schema != 1
                    || r.plan != plan.digest
                    || r.config != plan.config_digest
                    || r.matrix != plan.matrix_digest
                    || r.sequence != sequence
                    || !["start", "ok", "skipped", "failed"].contains(&r.event.as_str())
                    || !plan.steps.iter().any(|s| s.id == r.step)
                {
                    return Err(BoreaError::new(
                        "E_PLAN_STALE",
                        "journal belongs to another plan/config/matrix or has invalid records",
                    ));
                }
                sequence += 1;
            }
        }
        Ok(Self {
            file,
            plan: plan.digest.clone(),
            config: plan.config_digest.clone(),
            matrix: plan.matrix_digest.clone(),
            sequence,
        })
    }
    pub fn record(&mut self, step: &str, event: &str, detail: Option<&str>) -> Result<()> {
        use std::io::{Seek, SeekFrom};
        let r = Record {
            schema: 1,
            plan: self.plan.clone(),
            config: self.config.clone(),
            matrix: self.matrix.clone(),
            sequence: self.sequence,
            step: step.into(),
            event: event.into(),
            detail: detail.map(String::from),
        };
        let mut bytes = serde_json::to_vec(&r)?;
        bytes.push(b'\n');
        self.file.seek(SeekFrom::End(0))?;
        self.file.write_all(&bytes)?;
        self.file.sync_all()?;
        self.sequence += 1;
        Ok(())
    }
}
