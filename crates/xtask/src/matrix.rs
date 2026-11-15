//! F05 validated-cell record and release gate.
//!
//! `recipes/validated.json` lists the required cells whose e2e run passed in
//! CI, each tied to one `result.json` by digest. It is kept apart from
//! `recipes/matrix.toml` so recording a VM result does not change plan
//! digests. `matrix record` is the only writer, and it accepts only a passing
//! result for a required cell.
//!
//! - `matrix verify` (nightly): every record is well formed and names a
//!   required cell. With `--artifacts DIR`, every recorded cell that was re-run
//!   in DIR must have passed (regression check).
//! - `matrix verify --strict` (release): additionally every required cell has a
//!   record, and with `--artifacts DIR` each record's own result is present,
//!   digest-identical and complete. Without DIR a strict run cannot prove the
//!   artifacts and fails.

use borea_core::{BoreaError, Filesystem, Loader, Result, matrix::Matrix, recipe};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

/// Files an e2e result must list for its cell to count as validated.
const EVIDENCE: &[&str] = &["plan", "journal", "serial_log", "verify"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// `family/profile`, as in `result.json`.
    pub cell: String,
    pub root_fs: String,
    pub loader: String,
    /// Commit the CI run tested (40 hex digits).
    pub commit: String,
    /// GitHub Actions job URL that produced the result.
    pub run: String,
    /// Uploaded artifact name and id holding the result directory.
    pub artifact: String,
    pub artifact_id: u64,
    /// `borea_core::plan::digest` of the parsed `result.json`.
    pub result_digest: String,
    /// Digest of the cell's golden plan when the run happened. If the current
    /// golden plan differs, the recipe or planner changed and the record is
    /// stale (F06).
    pub golden_digest: String,
    /// UTC date of the run, `YYYY-MM-DD`.
    pub validated: String,
    pub release: String,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Validated {
    pub schema: u32,
    pub cells: Vec<Record>,
}

/// A required cell as the release contract (and `plan golden`) defines it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Required {
    pub cell: String,
    pub root_fs: String,
    pub loader: String,
    /// Digest of the cell's current golden plan; `None` when the cell is a
    /// refusal today.
    pub golden_digest: Option<String>,
}

fn fs_name(fs: Filesystem) -> &'static str {
    match fs {
        Filesystem::Btrfs => "btrfs",
        Filesystem::Ext4 => "ext4",
    }
}

/// Required cells with their resolved loader, in `cases()` order.
pub fn required() -> Result<Vec<Required>> {
    let m = Matrix::embedded()?;
    crate::cases()
        .into_iter()
        .map(|(f, profile, fs, l)| {
            let c = crate::case_config(f, &profile, fs, l)?;
            let loader = match borea_distro::loader(&c, &recipe::family(f)?, &m)? {
                Loader::Limine => "limine",
                Loader::Grub => "grub",
                Loader::Auto => return Err(BoreaError::invalid("loader resolved to auto")),
            };
            Ok(Required {
                cell: format!("{}/{profile}", f.as_str()),
                root_fs: fs_name(fs).into(),
                loader: loader.into(),
                golden_digest: crate::build_case(&c).ok().map(|p| p.digest),
            })
        })
        .collect()
}

fn path() -> PathBuf {
    crate::workspace().join("recipes/validated.json")
}

pub fn load() -> Result<Validated> {
    parse(&fs::read_to_string(path())?)
}

const FIELDS: &[&str] = &[
    "cell",
    "root_fs",
    "loader",
    "commit",
    "run",
    "artifact",
    "artifact_id",
    "result_digest",
    "golden_digest",
    "validated",
    "release",
];

fn parse(text: &str) -> Result<Validated> {
    let bad = |why: String| BoreaError::new("E_MATRIX", format!("recipes/validated.json: {why}"));
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| bad(e.to_string()))?;
    let top = v
        .as_object()
        .ok_or_else(|| bad("must be an object".into()))?;
    if top.keys().any(|k| k != "schema" && k != "cells") || top.get("schema") != Some(&2.into()) {
        return Err(bad("must be {\"schema\": 2, \"cells\": [...]}".into()));
    }
    let cells = top
        .get("cells")
        .and_then(|c| c.as_array())
        .ok_or_else(|| bad("cells must be an array".into()))?;
    let mut out = Vec::new();
    for c in cells {
        let o = c
            .as_object()
            .ok_or_else(|| bad("cell must be an object".into()))?;
        if o.len() != FIELDS.len() || !FIELDS.iter().all(|f| o.contains_key(*f)) {
            return Err(bad(format!("each cell has exactly: {}", FIELDS.join(", "))));
        }
        let s = |k: &str| {
            o[k].as_str()
                .map(str::to_string)
                .ok_or_else(|| bad(format!("{k} must be a string")))
        };
        out.push(Record {
            cell: s("cell")?,
            root_fs: s("root_fs")?,
            loader: s("loader")?,
            commit: s("commit")?,
            run: s("run")?,
            artifact: s("artifact")?,
            artifact_id: o["artifact_id"]
                .as_u64()
                .ok_or_else(|| bad("artifact_id must be a number".into()))?,
            result_digest: s("result_digest")?,
            golden_digest: s("golden_digest")?,
            validated: s("validated")?,
            release: s("release")?,
        });
    }
    Ok(Validated {
        schema: 2,
        cells: out,
    })
}

fn render_json(v: &Validated) -> Result<String> {
    let cells: Vec<_> = v
        .cells
        .iter()
        .map(|r| {
            serde_json::json!({
                "cell": r.cell, "root_fs": r.root_fs, "loader": r.loader,
                "commit": r.commit, "run": r.run, "artifact": r.artifact,
                "artifact_id": r.artifact_id, "result_digest": r.result_digest,
                "golden_digest": r.golden_digest,
                "validated": r.validated, "release": r.release,
            })
        })
        .collect();
    Ok(format!(
        "{}\n",
        serde_json::to_string_pretty(&serde_json::json!({"schema": 2, "cells": cells}))?
    ))
}

fn hex(s: &str, len: usize) -> bool {
    s.len() == len
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

/// Structural checks: shape, uniqueness, sorted order, and a required cell
/// with the cell's own storage and loader.
pub fn validate(v: &Validated, required: &[Required]) -> Result<()> {
    let bad = |cell: &str, why: &str| {
        Err(BoreaError::new(
            "E_MATRIX",
            format!("validated cell {cell}: {why}"),
        ))
    };
    let mut seen = BTreeMap::new();
    for r in &v.cells {
        let Some(req) = required.iter().find(|q| q.cell == r.cell) else {
            return bad(&r.cell, "is not a required cell");
        };
        if (r.root_fs.as_str(), r.loader.as_str()) != (req.root_fs.as_str(), req.loader.as_str()) {
            return bad(
                &r.cell,
                &format!("must be {} with {}", req.root_fs, req.loader),
            );
        }
        if !hex(&r.commit, 40) || !hex(&r.result_digest, 64) || !hex(&r.golden_digest, 64) {
            return bad(
                &r.cell,
                "commit, result_digest and golden_digest must be lowercase hex",
            );
        }
        if !r
            .run
            .starts_with("https://github.com/BoreaProj/borea/actions/runs/")
        {
            return bad(&r.cell, "run must be a BoreaProj/borea Actions URL");
        }
        if r.artifact.is_empty() || r.artifact_id == 0 || !date(&r.validated) {
            return bad(
                &r.cell,
                "artifact, artifact_id and validated date are required",
            );
        }
        if seen.insert(r.cell.as_str(), ()).is_some() {
            return bad(&r.cell, "is recorded twice");
        }
    }
    let order: Vec<_> = v
        .cells
        .iter()
        .map(|r| required.iter().position(|q| q.cell == r.cell))
        .collect();
    if order.windows(2).any(|w| w[0] > w[1]) {
        return Err(BoreaError::new(
            "E_MATRIX",
            "validated cells must follow the required-cell order; use matrix record",
        ));
    }
    Ok(())
}

/// Records whose cell plan changed since the run: the recipe or planner moved
/// on, so the VM result no longer proves the current plan.
pub fn stale<'a>(v: &'a Validated, required: &[Required]) -> Vec<&'a Record> {
    v.cells
        .iter()
        .filter(|r| {
            required
                .iter()
                .find(|q| q.cell == r.cell)
                .is_none_or(|q| q.golden_digest.as_deref() != Some(r.golden_digest.as_str()))
        })
        .collect()
}

/// One `result.json` found under an artifacts directory.
#[derive(Debug)]
struct Found {
    path: PathBuf,
    digest: String,
    value: serde_json::Value,
}

fn results(dir: &Path) -> Result<Vec<Found>> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in fs::read_dir(&d)? {
            let entry = entry?;
            let p = entry.path();
            let kind = entry.file_type()?;
            if kind.is_dir() {
                stack.push(p);
            } else if kind.is_file() && p.file_name().is_some_and(|n| n == "result.json") {
                let value: serde_json::Value = serde_json::from_str(&fs::read_to_string(&p)?)
                    .map_err(|e| BoreaError::new("E_MATRIX", format!("{}: {e}", p.display())))?;
                out.push(Found {
                    digest: borea_core::plan::digest(&value)?,
                    path: p,
                    value,
                });
            }
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

/// A result that passed and carries every evidence file beside it.
fn complete(f: &Found) -> Result<()> {
    let v = &f.value;
    let fail = |why: String| {
        Err(BoreaError::new(
            "E_MATRIX",
            format!("{}: {why}", f.path.display()),
        ))
    };
    if v["pass"] != true || !v["failed_stage"].is_null() {
        return fail("did not pass".into());
    }
    let dir = f.path.parent().expect("result.json has a parent");
    for key in EVIDENCE {
        let Some(name) = v["artifacts"][key].as_str() else {
            return fail(format!("lists no {key} artifact"));
        };
        if name.contains('/') || !dir.join(name).is_file() {
            return fail(format!("{key} artifact {name} is missing"));
        }
    }
    Ok(())
}

/// Create or replace the record for a passing `result.json`.
pub fn record(result: &Path, run: &str, artifact: &str, artifact_id: u64) -> Result<Record> {
    let value: serde_json::Value = serde_json::from_str(&fs::read_to_string(result)?)?;
    let found = Found {
        path: result.to_path_buf(),
        digest: borea_core::plan::digest(&value)?,
        value,
    };
    complete(&found)?;
    let v = &found.value;
    let text = |k: &str| -> Result<String> {
        v[k].as_str()
            .map(str::to_string)
            .ok_or_else(|| BoreaError::invalid(format!("result.json has no {k}")))
    };
    let dir = found
        .path
        .parent()
        .and_then(Path::file_name)
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let record = Record {
        cell: text("cell")?,
        root_fs: text("root_fs")?,
        loader: text("loader")?,
        commit: text("commit")?,
        run: run.into(),
        artifact: artifact.into(),
        artifact_id,
        result_digest: found.digest.clone(),
        golden_digest: text("golden_digest")?,
        // The run directory is the UTC start time, YYYYMMDDTHHMMSSZ.
        validated: match dir.get(..8) {
            Some(d) if d.bytes().all(|b| b.is_ascii_digit()) => {
                format!("{}-{}-{}", &d[..4], &d[4..6], &d[6..])
            }
            _ => {
                return Err(BoreaError::invalid(
                    "result.json is not in an e2e run directory",
                ));
            }
        },
        release: text("release")?,
    };
    let required = required()?;
    let mut all = load()?;
    all.cells.retain(|r| r.cell != record.cell);
    all.cells.push(record.clone());
    all.cells
        .sort_by_key(|r| required.iter().position(|q| q.cell == r.cell));
    validate(&all, &required)?;
    fs::write(path(), render_json(&all)?)?;
    Ok(record)
}

/// Release and nightly gate. Returns the summary line on success.
pub fn verify(strict: bool, artifacts: Option<&Path>) -> Result<String> {
    verify_with(&load()?, &required()?, strict, artifacts)
}

fn verify_with(
    v: &Validated,
    required: &[Required],
    strict: bool,
    artifacts: Option<&Path>,
) -> Result<String> {
    validate(v, required)?;
    let found = match artifacts {
        Some(dir) => Some(results(dir)?),
        None => None,
    };
    let mut errors = Vec::new();
    if let Some(found) = &found {
        for r in &v.cells {
            let runs: Vec<_> = found.iter().filter(|f| f.value["cell"] == r.cell).collect();
            // Nightly: a recorded cell that was re-run must still pass.
            for f in &runs {
                if let Err(e) = complete(f) {
                    errors.push(format!("{}: regression: {}", r.cell, e.message));
                }
            }
            if strict {
                match runs.iter().find(|f| f.digest == r.result_digest) {
                    None => errors.push(format!(
                        "{}: recorded result {} not found under {}",
                        r.cell,
                        &r.result_digest[..12],
                        artifacts.unwrap_or(Path::new("")).display()
                    )),
                    Some(f) => {
                        if f.value["commit"] != r.commit.as_str() {
                            errors.push(format!("{}: result commit differs from record", r.cell));
                        }
                    }
                }
            }
        }
    }
    let stale = stale(v, required);
    let missing: Vec<_> = required
        .iter()
        .filter(|q| !v.cells.iter().any(|r| r.cell == q.cell))
        .map(|q| q.cell.as_str())
        .collect();
    if strict {
        if found.is_none() {
            errors.push(
                "strict verify needs --artifacts DIR holding each record's e2e result".into(),
            );
        }
        if !missing.is_empty() {
            errors.push(format!(
                "{} required cells have no validated e2e result:\n  {}",
                missing.len(),
                missing.join("\n  ")
            ));
        }
        if !stale.is_empty() {
            errors.push(format!(
                "{} validated cells are stale (their plan changed since the run; re-run and re-record):\n  {}",
                stale.len(),
                stale
                    .iter()
                    .map(|r| r.cell.as_str())
                    .collect::<Vec<_>>()
                    .join("\n  ")
            ));
        }
    }
    let summary = format!(
        "matrix verify: {}/{} required cells validated ({} stale)",
        v.cells.len() - stale.len(),
        required.len(),
        stale.len()
    );
    if errors.is_empty() {
        Ok(summary)
    } else {
        Err(BoreaError::new(
            "E_MATRIX_UNTESTED",
            format!("release gate failed: {summary}\n{}", errors.join("\n")),
        ))
    }
}

/// The validated-cell section appended to `MATRIX.generated.md`.
pub fn render_section(v: &Validated, required: &[Required]) -> String {
    let stale = stale(v, required);
    let mut text = format!(
        "\n## VM-validated cells ({}/{} required, {} stale)\n\nA cell is validated only by a passing e2e run in CI, recorded with `cargo xtask matrix record` in `recipes/validated.json`. A record is stale when the cell's golden plan has changed since the run; it then counts as unvalidated. Package tiers above are not VM results.\n\n",
        v.cells.len() - stale.len(),
        required.len(),
        stale.len()
    );
    if v.cells.is_empty() {
        text += "None yet.\n";
        return text;
    }
    text += "| Cell | Root fs | Loader | Status | Commit | Date | Run |\n| --- | --- | --- | --- | --- | --- | --- |\n";
    for r in &v.cells {
        text += &format!(
            "| {} | {} | {} | {} | `{}` | {} | [{} #{}]({}) |\n",
            r.cell,
            r.root_fs,
            r.loader,
            if stale.iter().any(|s| s.cell == r.cell) {
                "stale"
            } else {
                "current"
            },
            &r.commit[..7],
            r.validated,
            r.artifact,
            r.artifact_id,
            r.run
        );
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req() -> Vec<Required> {
        required().unwrap()
    }

    fn rec(cell: &str) -> Record {
        Record {
            cell: cell.into(),
            root_fs: "btrfs".into(),
            loader: "limine".into(),
            commit: "8".repeat(40),
            run: "https://github.com/BoreaProj/borea/actions/runs/1/job/2".into(),
            artifact: "e2e-arch-minimal".into(),
            artifact_id: 3,
            result_digest: "d".repeat(64),
            golden_digest: required()
                .unwrap()
                .into_iter()
                .find(|q| q.cell == cell)
                .and_then(|q| q.golden_digest)
                .unwrap_or_else(|| "e".repeat(64)),
            validated: "2026-11-14".into(),
            release: "B-0.3".into(),
        }
    }

    fn run_dir(root: &Path, cell: &str, pass: bool, evidence: bool) -> serde_json::Value {
        let dir = root.join(cell).join("20261114T222838Z");
        fs::create_dir_all(&dir).unwrap();
        let mut artifacts = serde_json::Map::new();
        for key in EVIDENCE {
            let name = format!("{key}.out");
            if evidence {
                fs::write(dir.join(&name), "x").unwrap();
            }
            artifacts.insert((*key).into(), name.into());
        }
        let v = serde_json::json!({
            "cell": cell, "root_fs": "btrfs", "loader": "limine",
            "commit": "8".repeat(40), "pass": pass,
            "failed_stage": if pass { serde_json::Value::Null } else { "boot".into() },
            "artifacts": artifacts, "release": "B-0.3",
            "golden_digest": "e".repeat(64),
        });
        fs::write(dir.join("result.json"), v.to_string()).unwrap();
        v
    }

    #[test]
    fn required_cells_match_the_contract() {
        let r = req();
        assert_eq!(r.len(), 29);
        assert_eq!(r[0].cell, "arch/minimal");
        assert_eq!(
            (r[0].root_fs.as_str(), r[0].loader.as_str()),
            ("btrfs", "limine")
        );
        let grub = r.iter().find(|q| q.cell == "arch/minimal-grub").unwrap();
        assert_eq!(grub.loader, "grub");
        let ext4 = r.iter().find(|q| q.cell == "debian/minimal-ext4").unwrap();
        assert_eq!(
            (ext4.root_fs.as_str(), ext4.loader.as_str()),
            ("ext4", "grub")
        );
    }

    #[test]
    fn records_are_validated() {
        let r = req();
        let ok = Validated {
            schema: 2,
            cells: vec![rec("arch/minimal")],
        };
        assert!(validate(&ok, &r).is_ok());
        for broken in [
            Record {
                cell: "arch/everything".into(),
                ..rec("arch/minimal")
            },
            Record {
                loader: "grub".into(),
                ..rec("arch/minimal")
            },
            Record {
                commit: "abc".into(),
                ..rec("arch/minimal")
            },
            Record {
                run: "https://example.com/run".into(),
                ..rec("arch/minimal")
            },
            Record {
                validated: "14/11/2026".into(),
                ..rec("arch/minimal")
            },
        ] {
            let v = Validated {
                schema: 2,
                cells: vec![broken],
            };
            assert!(validate(&v, &r).is_err());
        }
        let dup = Validated {
            schema: 2,
            cells: vec![rec("arch/minimal"), rec("arch/minimal")],
        };
        assert!(validate(&dup, &r).is_err());
        assert!(parse(r#"{"schema":1,"cells":[]}"#).is_err());
        assert!(parse(r#"{"schema":2,"cells":[],"extra":1}"#).is_err());
        let text = render_json(&ok).unwrap();
        assert_eq!(parse(&text).unwrap(), ok);
        assert!(parse(&text.replace("\"release\"", "\"extra\": 1, \"release\"")).is_err());
    }

    #[test]
    fn results_need_a_pass_and_every_evidence_file() {
        let d = tempfile::tempdir().unwrap();
        run_dir(d.path(), "arch/minimal", true, true);
        let found = results(d.path()).unwrap();
        assert!(complete(&found[0]).is_ok());
        let d = tempfile::tempdir().unwrap();
        run_dir(d.path(), "arch/minimal", true, false);
        assert!(complete(&results(d.path()).unwrap()[0]).is_err());
        let d = tempfile::tempdir().unwrap();
        run_dir(d.path(), "arch/minimal", false, true);
        assert!(complete(&results(d.path()).unwrap()[0]).is_err());
    }

    #[test]
    fn digest_ties_a_record_to_one_result() {
        let d = tempfile::tempdir().unwrap();
        let v = run_dir(d.path(), "arch/minimal", true, true);
        let found = results(d.path()).unwrap();
        assert_eq!(found[0].digest, borea_core::plan::digest(&v).unwrap());
        let mut other = v.clone();
        other["commit"] = "9".repeat(40).into();
        assert_ne!(found[0].digest, borea_core::plan::digest(&other).unwrap());
    }

    #[test]
    fn gate_requires_matching_evidence() {
        let r = req();
        let d = tempfile::tempdir().unwrap();
        let v = run_dir(d.path(), "arch/minimal", true, true);
        let digest = borea_core::plan::digest(&v).unwrap();
        let recorded = Validated {
            schema: 2,
            cells: vec![Record {
                result_digest: digest,
                ..rec("arch/minimal")
            }],
        };
        // Nightly: the recorded cell was re-run and passed.
        assert!(verify_with(&recorded, &r, false, Some(d.path())).is_ok());
        // Release: 28 required cells have no record, so the gate fails.
        let e = verify_with(&recorded, &r, true, Some(d.path())).unwrap_err();
        assert!(
            e.message
                .contains("28 required cells have no validated e2e result")
        );
        assert!(!e.message.contains("arch/minimal:"));
        // Release without artifacts cannot prove anything.
        let e = verify_with(&recorded, &r, true, None).unwrap_err();
        assert!(e.message.contains("needs --artifacts"));
        // A record whose own result is absent fails strict verify.
        let other = Validated {
            schema: 2,
            cells: vec![rec("arch/minimal")],
        };
        let e = verify_with(&other, &r, true, Some(d.path())).unwrap_err();
        assert!(e.message.contains("arch/minimal: recorded result"));
        // Nightly: a recorded cell that now fails is a regression.
        let bad = tempfile::tempdir().unwrap();
        run_dir(bad.path(), "arch/minimal", false, true);
        let e = verify_with(&recorded, &r, false, Some(bad.path())).unwrap_err();
        assert!(e.message.contains("arch/minimal: regression"));
        // An unrecorded cell failing is not a regression.
        assert!(
            verify_with(
                &Validated {
                    schema: 2,
                    cells: vec![]
                },
                &r,
                false,
                Some(bad.path())
            )
            .is_ok()
        );
    }

    #[test]
    fn records_go_stale_when_the_golden_plan_changes() {
        let r = req();
        let current = rec("arch/minimal");
        let old = Record {
            golden_digest: "0".repeat(64),
            root_fs: "ext4".into(),
            ..rec("arch/minimal-ext4")
        };
        let v = Validated {
            schema: 2,
            cells: vec![current, old],
        };
        let stale: Vec<_> = stale(&v, &r).iter().map(|s| s.cell.clone()).collect();
        assert_eq!(stale, ["arch/minimal-ext4"]);
        let e = verify_with(&v, &r, true, None).unwrap_err();
        assert!(e.message.contains("1 validated cells are stale"));
        assert!(
            e.message
                .contains("1/29 required cells validated (1 stale)")
        );
        assert!(render_section(&v, &r).contains("| arch/minimal-ext4 | ext4 | limine | stale |"));
        // A stale record does not fail nightly or matrix check.
        assert!(verify_with(&v, &r, false, None).is_ok());
    }

    #[test]
    fn render_lists_records() {
        let empty = render_section(
            &Validated {
                schema: 2,
                cells: vec![],
            },
            &req(),
        );
        assert!(empty.contains("(0/29 required, 0 stale)") && empty.contains("None yet."));
        let one = render_section(
            &Validated {
                schema: 2,
                cells: vec![rec("arch/minimal")],
            },
            &req(),
        );
        assert!(
            one.contains("| arch/minimal | btrfs | limine | current | `8888888` | 2026-11-14 |")
        );
    }
}
