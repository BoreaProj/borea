use borea_core::{defaults::*, *};
use std::{fs, io::Read, path::PathBuf};
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(e.exit_code());
    }
}
fn run() -> Result<()> {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() || args == ["--help"] {
        println!(
            "borea-defaults status|restore ID...|note-install --window update|user [--policy FILE]\ntrim/enforce await verified upstream pins; no removals are performed."
        );
        return Ok(());
    }
    let cmd = args.remove(0);
    let mut policy = PathBuf::from("/var/lib/borea/defaults.json");
    let mut window = None;
    let mut ids = Vec::new();
    let mut iter = args.into_iter();
    while let Some(a) = iter.next() {
        match a.as_str() {
            "--policy" => {
                policy = iter
                    .next()
                    .ok_or_else(|| BoreaError::invalid("--policy needs FILE"))?
                    .into()
            }
            "--window" => {
                window = Some(
                    iter.next()
                        .ok_or_else(|| BoreaError::invalid("--window needs update or user"))?,
                )
            }
            "--json" => {}
            _ if a.starts_with('-') => return Err(BoreaError::invalid("unknown option")),
            _ => ids.push(a),
        }
    }
    if ["trim", "enforce"].contains(&cmd.as_str()) {
        return Err(BoreaError::unsupported(
            "removals await verified Omarchy commit/update-window integration; use status or restore",
        ));
    }
    if !["status", "restore", "note-install"].contains(&cmd.as_str()) {
        return Err(BoreaError::invalid("unknown defaults command"));
    }
    let metadata = fs::symlink_metadata(&policy)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(BoreaError::invalid("policy must be a regular file"));
    }
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(cmd != "status")
        .open(&policy)?;
    file.try_lock()
        .map_err(|_| BoreaError::new("E_POLICY_LOCK", "policy is already in use"))?;
    let mut text = String::new();
    file.read_to_string(&mut text)?;
    let mut p: DefaultsPolicy = serde_json::from_str(&text)?;
    p.validate()?;
    match cmd.as_str() {
        "status" => {
            if !ids.is_empty() || window.is_some() {
                return Err(BoreaError::invalid("status accepts only --policy/--json"));
            }
            println!("{}", serde_json::to_string_pretty(&p)?);
            return Ok(());
        }
        "restore" => {
            if ids.is_empty() || window.is_some() {
                return Err(BoreaError::invalid("restore needs ids"));
            }
            p.restore(&ids)?;
        }
        "note-install" => {
            if !ids.is_empty() {
                return Err(BoreaError::invalid("note-install reads packages on stdin"));
            }
            let update = match window.as_deref() {
                Some("update") => true,
                Some("user") => false,
                _ => {
                    return Err(BoreaError::unsupported(
                        "update window cannot yet be detected automatically; fixture recording requires --window update or user",
                    ));
                }
            };
            let mut stdin = String::new();
            std::io::stdin().read_to_string(&mut stdin)?;
            let packages = stdin
                .split_whitespace()
                .map(String::from)
                .collect::<Vec<_>>();
            p.note_install(&packages, update);
        }
        _ => unreachable!(),
    }
    // Hold an exclusive lock and sync policy updates. A backup permits recovery from interrupted writes.
    use std::io::{Seek, SeekFrom, Write};
    let mut backup = policy.clone();
    backup.set_extension("json.bak");
    fs::write(&backup, &text)?;
    let bytes = serde_json::to_vec_pretty(&p)?;
    file.seek(SeekFrom::Start(0))?;
    file.write_all(&bytes)?;
    file.set_len(bytes.len() as u64)?;
    file.sync_all()?;
    println!("{}", serde_json::to_string_pretty(&p)?);
    Ok(())
}
