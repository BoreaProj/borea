use borea_core::{matrix::Matrix, recipe, *};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(e.exit_code());
    }
}
#[derive(Default)]
struct Args {
    values: BTreeMap<String, String>,
    flags: Vec<String>,
    positionals: Vec<String>,
}
impl Args {
    fn parse(args: Vec<String>, values: &[&str], flags: &[&str]) -> Result<Self> {
        let mut out = Self::default();
        let mut args = args.into_iter();
        while let Some(a) = args.next() {
            if let Some(k) = a.strip_prefix("--") {
                if values.contains(&k) {
                    let v = args
                        .next()
                        .filter(|v| !v.starts_with("--"))
                        .ok_or_else(|| BoreaError::invalid(format!("--{k} needs a value")))?;
                    if out.values.insert(k.into(), v).is_some() {
                        return Err(BoreaError::invalid(format!("duplicate --{k}")));
                    }
                } else if flags.contains(&k) {
                    if out.flags.iter().any(|f| f == k) {
                        return Err(BoreaError::invalid(format!("duplicate --{k}")));
                    }
                    out.flags.push(k.into());
                } else {
                    return Err(BoreaError::invalid(format!("unknown option --{k}")));
                }
            } else if a.starts_with('-') {
                return Err(BoreaError::invalid(format!("unknown option {a}")));
            } else {
                out.positionals.push(a);
            }
        }
        Ok(out)
    }
    fn required(&self, k: &str) -> Result<&str> {
        self.values
            .get(k)
            .map(String::as_str)
            .ok_or_else(|| BoreaError::invalid(format!("--{k} required")))
    }
    fn value(&self, k: &str) -> Option<&str> {
        self.values.get(k).map(String::as_str)
    }
    fn flag(&self, k: &str) -> bool {
        self.flags.iter().any(|s| s == k)
    }
    fn no_positionals(&self) -> Result<()> {
        if self.positionals.is_empty() {
            Ok(())
        } else {
            Err(BoreaError::invalid("unexpected positional argument"))
        }
    }
}
fn output(value: &serde_json::Value, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        match value {
            serde_json::Value::Array(items) => {
                for item in items {
                    println!("{}", serde_json::to_string_pretty(item)?);
                }
            }
            _ => println!("{}", serde_json::to_string_pretty(value)?),
        }
    }
    Ok(())
}
fn load_plan(path: &str) -> Result<Plan> {
    let p: Plan = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    p.validate()?;
    Ok(p)
}
fn run() -> Result<()> {
    let mut raw = std::env::args().skip(1).collect::<Vec<_>>();
    if raw.is_empty() || raw == ["--help"] || raw == ["help"] {
        println!(
            "borea {} — experimental installer planning, family runtimes and fixture tools\n\nCommands:\n  doctor --family FAMILY [--cache DIR] [--json]\n  image create|apply --family FAMILY --manifest FILE (disposable storage only)\n  runtime list|fetch|verify|probe|exec [--family FAMILY] [--cache DIR]\n  catalogue --family FAMILY [--json]\n  explain COMPONENT --family FAMILY\n  plan --config FILE --out FILE [--dry-run] [--fixture --identity FILE]\n  apply PLAN --dry-run\n  apply PLAN --yes --device DEVICE (live execution gated)\n  apply PLAN --fixture-root DIR --journal FILE [--resume]\n  verify PLAN --root DIR | --ssh USER@HOST [--json]\n  status --journal FILE\n  omarchy configure --config FILE [--json]\n  omarchy trim --list | ID... --dry-run\n  omarchy defaults status --policy FILE\n\nNo VM cells are supported yet. See docs/IMPLEMENTATION.md.",
            version()
        );
        return Ok(());
    }
    if raw == ["--version"] {
        println!("borea {}", version());
        return Ok(());
    }
    let cmd = raw.remove(0);
    if cmd == "image" {
        return image(raw);
    }
    if cmd == "runtime" {
        return runtime(raw);
    }
    if cmd == "omarchy" {
        return omarchy(raw);
    }
    let (values, flags): (&[&str], &[&str]) = match cmd.as_str() {
        "doctor" => (&["family", "cache"], &["json"]),
        "catalogue" | "explain" => (&["family"], &["json"]),
        "plan" => (
            &["config", "out", "identity"],
            &[
                "json",
                "dry-run",
                "fixture",
                "allow-source",
                "deny-third-party",
            ],
        ),
        "apply" => (
            &["device", "fixture-root", "journal"],
            &["json", "dry-run", "yes", "resume"],
        ),
        "verify" => (&["root", "ssh"], &["json"]),
        "status" => (&["journal"], &["json"]),
        _ => return Err(BoreaError::invalid("unknown command; run borea --help")),
    };
    let a = Args::parse(raw, values, flags)?;
    let json = a.flag("json");
    match cmd.as_str() {
        "doctor" => {
            a.no_positionals()?;
            let f = FamilyId::parse(a.required("family")?)?;
            let cache = a
                .value("cache")
                .map(PathBuf::from)
                .map(Ok)
                .unwrap_or_else(borea_run::runtime::default_cache)?;
            let lock = borea_core::runtime::RuntimeLock::embedded()?;
            let spec = lock.get(f)?;
            let result = borea_run::runtime::probe(spec, &cache);
            let uefi = Path::new("/sys/firmware/efi").is_dir();
            let runtime = match &result {
                Ok(s) => serde_json::to_value(s)?,
                Err(e) => {
                    serde_json::json!({"verified":false,"error":e.to_string(),"remedy":format!("borea runtime fetch --family {} --cache {}",f.as_str(),cache.display())})
                }
            };
            let passed = uefi && result.is_ok();
            output(
                &serde_json::json!({"schema":2,"family":f,"uefi":uefi,"runtime":runtime,"passed":passed,"live_apply_enabled":false}),
                json,
            )?;
            if !passed {
                return Err(BoreaError::unsupported(
                    "preflight incomplete; fetch/probe the Borea runtime and use UEFI for installation; disk execution remains a separate milestone",
                ));
            }
        }
        "catalogue" => {
            a.no_positionals()?;
            let f = FamilyId::parse(a.required("family")?)?;
            let recipes = recipe::recipes()?;
            let cells=Matrix::embedded()?.cells.into_iter().filter(|c|c.family==f).map(|c|serde_json::json!({"component":c.component,"tier":c.tier,"reason":c.reason,"remedy":c.remedy,"evidence":c.evidence,"recipe_available":recipes.get(&c.component).is_some_and(|r|r.packages.contains_key(&f)),"vm_tested":false,"supported":false})).collect::<Vec<_>>();
            output(&serde_json::json!(cells), json)?;
        }
        "explain" => {
            if a.positionals.len() != 1 {
                return Err(BoreaError::invalid("explain needs one COMPONENT"));
            }
            let f = FamilyId::parse(a.required("family")?)?;
            let matrix = Matrix::embedded()?;
            let c = matrix.cell(f, &a.positionals[0])?;
            output(&serde_json::to_value(c)?, json)?;
        }
        "plan" => {
            a.no_positionals()?;
            let c = Config::parse(&std::fs::read_to_string(a.required("config")?)?)?;
            let m = Matrix::embedded()?;
            let f = recipe::family(c.target.family)?;
            let l = borea_distro::loader(&c, &f, &m)?;
            let s = borea_session::select(
                &c,
                &m,
                l,
                a.flag("allow-source"),
                a.flag("deny-third-party"),
            )?;
            let disk = if a.flag("fixture") {
                serde_json::from_str(&std::fs::read_to_string(a.required("identity")?)?)?
            } else {
                if a.value("identity").is_some() {
                    return Err(BoreaError::invalid("--identity requires --fixture"));
                }
                let mut d = borea_distro::device::identity(&c.disk.device)?;
                d.path = c.disk.device.clone();
                d
            };
            let mut p = borea_distro::planner::build(
                &c,
                disk,
                borea_distro::planner::Intent {
                    components: s.components,
                    greeter: s.greeter,
                    session: s.session,
                    autostart: s.autostart,
                },
            )?;
            if a.flag("fixture") {
                p.notes
                    .push("Fixture identity: this plan is only for inspection and tests".into());
                p.seal()?;
            }
            let out = PathBuf::from(a.required("out")?);
            for input in [a.value("config"), a.value("identity")]
                .into_iter()
                .flatten()
            {
                if Path::new(input) == out
                    || std::fs::canonicalize(input)
                        .ok()
                        .zip(std::fs::canonicalize(&out).ok())
                        .is_some_and(|(i, o)| i == o)
                {
                    return Err(BoreaError::invalid("--out must differ from every input"));
                }
            }
            // Planning writes exactly the requested output file, without mkdir or temp files.
            use std::{io::Write, os::unix::fs::OpenOptionsExt};
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&out)?;
            file.write_all(p.json()?.as_bytes())?;
            file.sync_all()?;
            if a.flag("dry-run") {
                output(&serde_json::to_value(borea_run::dry_run(&p)?)?, json)?;
            } else {
                output(
                    &serde_json::json!({"plan":out,"digest":p.digest,"steps":p.steps.len(),"executed":false}),
                    json,
                )?;
            }
        }
        "apply" => {
            if a.positionals.len() != 1 {
                return Err(BoreaError::invalid("apply needs one plan path"));
            }
            let p = load_plan(&a.positionals[0])?;
            if p.matrix_digest != digest(&Matrix::embedded()?)? {
                return Err(BoreaError::new("E_PLAN_STALE", "matrix changed; re-plan"));
            }
            if a.flag("dry-run") {
                if a.flag("resume")
                    || a.value("fixture-root").is_some()
                    || a.value("journal").is_some()
                {
                    return Err(BoreaError::invalid(
                        "dry-run cannot be combined with execution options",
                    ));
                }
                output(&serde_json::to_value(borea_run::dry_run(&p)?)?, json)?;
            } else if let Some(root) = a.value("fixture-root") {
                if a.flag("yes") || a.value("device").is_some() {
                    return Err(BoreaError::invalid(
                        "fixture execution cannot use live confirmation flags",
                    ));
                }
                borea_run::apply_fixture(
                    &p,
                    Path::new(root),
                    Path::new(a.required("journal")?),
                    a.flag("resume"),
                )?;
                output(
                    &serde_json::json!({"completed":true,"mode":"fixture"}),
                    json,
                )?;
            } else {
                borea_run::apply_live(&p, a.flag("yes"), Path::new(a.required("device")?))?;
            }
        }
        "verify" => {
            if a.positionals.len() != 1 || a.value("root").is_some() == a.value("ssh").is_some() {
                return Err(BoreaError::invalid(
                    "verify needs PLAN and exactly one of --root/--ssh",
                ));
            }
            let p = load_plan(&a.positionals[0])?;
            let r = if let Some(root) = a.value("root") {
                borea_run::verify::offline(&p, Path::new(root))?
            } else {
                borea_run::verify::ssh(&p, a.required("ssh")?)?
            };
            output(&serde_json::to_value(&r)?, json)?;
            if !r.passed {
                return Err(BoreaError::new("E_VERIFY", "one or more checks failed"));
            }
        }
        "status" => {
            a.no_positionals()?;
            let content = std::fs::read_to_string(a.required("journal")?)?;
            if !content.is_empty() && !content.ends_with('\n') {
                return Err(BoreaError::new("E_JOURNAL", "truncated journal"));
            }
            let records = content
                .lines()
                .map(serde_json::from_str::<borea_run::journal::Record>)
                .collect::<std::result::Result<Vec<_>, _>>()?;
            output(&serde_json::to_value(records)?, json)?;
        }
        _ => unreachable!(),
    }
    Ok(())
}
fn omarchy(mut raw: Vec<String>) -> Result<()> {
    if raw.first().is_some_and(|s| s == "configure") {
        raw.remove(0);
        let a = Args::parse(raw, &["config"], &["json"])?;
        a.no_positionals()?;
        let c = Config::parse(&std::fs::read_to_string(a.required("config")?)?)?;
        if c.session.kind != "omarchy" {
            return Err(BoreaError::invalid(
                "configure requires an Omarchy profile config",
            ));
        }
        let selection = c.omarchy.unwrap_or_default().selection()?;
        return output(
            &serde_json::json!({"schema":1,"family":c.target.family,"executed":false,"selection":selection}),
            a.flag("json"),
        );
    }

    if raw.first().is_some_and(|s| s == "defaults") {
        raw.remove(0);
        let status =
            std::process::Command::new(std::env::current_exe()?.with_file_name("borea-defaults"))
                .args(raw)
                .status()?;
        if !status.success() {
            return Err(BoreaError::new("E_STEP", "borea-defaults failed"));
        }
        return Ok(());
    }
    if raw.first().is_none_or(|s| s != "trim") {
        return Err(BoreaError::invalid(
            "use omarchy configure, omarchy trim or omarchy defaults",
        ));
    }
    raw.remove(0);
    let a = Args::parse(raw, &["group"], &["list", "dry-run", "json"])?;
    if a.flag("list") {
        return output(
            &serde_json::to_value(borea_core::defaults::CATALOG)?,
            a.flag("json"),
        );
    }
    let mut ids = a.positionals.clone();
    if let Some(group) = a.value("group") {
        if !["apps", "web-apps", "developer-tools"].contains(&group) {
            return Err(BoreaError::invalid("unknown trim group"));
        }
        ids.extend(
            borea_core::defaults::CATALOG
                .iter()
                .filter(|i| i.group == group)
                .map(|i| i.id.to_string()),
        );
    }
    ids.sort();
    ids.dedup();
    if ids.is_empty() {
        return Err(BoreaError::invalid("supply trim ids or --group"));
    }
    let preview = borea_run::omarchy::trim_preview(&ids)?;
    if !a.flag("dry-run") {
        borea_run::omarchy::enforce_gate()?;
    }
    output(&serde_json::json!(preview), a.flag("json"))
}

fn runtime(mut raw: Vec<String>) -> Result<()> {
    if raw.is_empty() || raw == ["--help"] {
        println!(
            "borea runtime list [--json]\nborea runtime fetch --family FAMILY [--cache DIR] [--archive FILE] [--offline] [--json]\nborea runtime verify|probe --family FAMILY [--cache DIR] [--json]\nborea runtime exec --family FAMILY [--cache DIR] [--network] -- COMMAND [ARGS...]\nRuntime execution is read-only and has no host disk/home/package-state mounts."
        );
        return Ok(());
    }
    let sub = raw.remove(0);
    let trailing = if let Some(i) = raw.iter().position(|s| s == "--") {
        let cmd = raw.split_off(i + 1);
        raw.pop();
        cmd
    } else {
        Vec::new()
    };
    let (values, flags): (&[&str], &[&str]) = match sub.as_str() {
        "list" => (&[], &["json"]),
        "fetch" => (&["family", "cache", "archive"], &["json", "offline"]),
        "verify" | "probe" => (&["family", "cache"], &["json"]),
        "exec" => (&["family", "cache"], &["network"]),
        _ => {
            return Err(BoreaError::invalid(
                "unknown runtime command; use runtime --help",
            ));
        }
    };
    let a = Args::parse(raw, values, flags)?;
    a.no_positionals()?;
    if sub != "exec" && !trailing.is_empty() {
        return Err(BoreaError::invalid("only runtime exec accepts a command"));
    }
    let lock = borea_core::runtime::RuntimeLock::embedded()?;
    if sub == "list" {
        return output(&serde_json::to_value(&lock)?, a.flag("json"));
    }
    let f = FamilyId::parse(a.required("family")?)?;
    let spec = lock.get(f)?;
    let cache = a
        .value("cache")
        .map(PathBuf::from)
        .map(Ok)
        .unwrap_or_else(borea_run::runtime::default_cache)?;
    if sub == "exec" {
        borea_run::runtime::run(spec, &cache, &trailing, a.flag("network"))?;
        return Ok(());
    }
    let state = match sub.as_str() {
        "fetch" => borea_run::runtime::fetch(
            spec,
            &cache,
            a.value("archive").map(Path::new),
            a.flag("offline"),
        )?,
        "verify" => borea_run::runtime::verify(spec, &cache)?,
        "probe" => borea_run::runtime::probe(spec, &cache)?,
        _ => unreachable!(),
    };
    output(&serde_json::to_value(state)?, a.flag("json"))
}

fn image(mut raw: Vec<String>) -> Result<()> {
    if raw.is_empty() {
        return Err(BoreaError::invalid("image create|apply required"));
    }
    let cmd = raw.remove(0);
    let a = Args::parse(
        raw,
        &[
            "image", "manifest", "fs", "size-gib", "family", "cache", "journal",
        ],
        &["yes", "resume", "json"],
    )?;
    a.no_positionals()?;
    let lock = borea_core::runtime::RuntimeLock::embedded()?;
    let family = FamilyId::parse(a.required("family")?)?;
    let spec = lock.get(family)?;
    match cmd.as_str() {
        "create" => {
            let size = a
                .value("size-gib")
                .unwrap_or("20")
                .parse::<u64>()
                .map_err(|_| BoreaError::invalid("invalid --size-gib"))?;
            let bytes = size
                .checked_mul(1024 * 1024 * 1024)
                .ok_or_else(|| BoreaError::invalid("image size overflow"))?;
            let plan = borea_run::image::create(
                Path::new(a.required("image")?),
                Path::new(a.required("manifest")?),
                a.value("fs").unwrap_or("btrfs"),
                bytes,
                spec,
            )?;
            output(&serde_json::to_value(plan)?, true)
        }
        "apply" => {
            let plan = serde_json::from_slice::<borea_run::image::ImagePlan>(&std::fs::read(
                a.required("manifest")?,
            )?)?;
            let cache = a
                .value("cache")
                .map(PathBuf::from)
                .map(Ok)
                .unwrap_or_else(borea_run::runtime::default_cache)?;
            borea_run::image::apply(
                &plan,
                spec,
                &cache,
                Path::new(a.required("journal")?),
                a.flag("resume"),
                a.flag("yes"),
            )?;
            output(
                &serde_json::json!({"schema":1,"storage_verified":true,"bootable":false}),
                true,
            )
        }
        _ => Err(BoreaError::invalid("unknown image command")),
    }
}
