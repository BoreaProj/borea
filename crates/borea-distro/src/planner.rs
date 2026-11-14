use crate::*;
use borea_core::{
    matrix::Matrix,
    recipe::{self, Recipe},
};
use std::{collections::BTreeSet, path::PathBuf};
use uuid::Uuid;
pub struct Intent {
    pub components: Vec<String>,
    pub greeter: String,
    pub session: Option<Recipe>,
    pub autostart: Option<String>,
}
struct Builder {
    steps: Vec<Step>,
}
impl Builder {
    fn add(&mut self, id: &str, phase: Phase, action: Action, probe: Probe, danger: Danger) {
        let needs = self
            .steps
            .last()
            .map(|s| vec![s.id.clone()])
            .unwrap_or_default();
        self.steps.push(Step {
            id: id.into(),
            title: id.replace('.', " "),
            action,
            needs,
            probe,
            danger,
            timeout_s: if phase == Phase::Bootstrap { 3600 } else { 600 },
            phase,
        });
    }
    fn cmd(
        &mut self,
        id: &str,
        phase: Phase,
        args: Vec<String>,
        chroot: bool,
        probe: Probe,
        danger: Danger,
    ) {
        self.add(
            id,
            phase,
            Action::Command {
                argv: args,
                chroot,
                stdin: None,
            },
            probe,
            danger,
        );
    }
    fn file(&mut self, id: &str, phase: Phase, path: &str, contents: String, mode: u32) {
        self.add(
            id,
            phase,
            Action::WriteFile {
                path: path.into(),
                contents: contents.clone(),
                mode,
            },
            Probe::FileContains {
                path: path.into(),
                needle: contents,
            },
            Danger::Mutating,
        );
    }
    fn mkdir(&mut self, id: &str, phase: Phase, path: &str) {
        self.add(
            id,
            phase,
            Action::Mkdir {
                path: path.into(),
                mode: 0o755,
            },
            Probe::FileExists { path: path.into() },
            Danger::Mutating,
        );
    }
}
fn argv(values: &[&str]) -> Vec<String> {
    values.iter().map(|s| (*s).into()).collect()
}
fn cmd_probe(args: &[&str], chroot: bool) -> Probe {
    Probe::CommandOk {
        argv: argv(args),
        chroot,
    }
}
pub fn build(c: &Config, disk: DeviceIdentity, intent: Intent) -> Result<Plan> {
    c.validate()?;
    let matrix = Matrix::embedded()?;
    let family = recipe::family(c.target.family)?;
    let resolved = crate::loader(c, &family, &matrix)?;
    if disk.path != c.disk.device {
        return Err(BoreaError::new(
            "E_DISK_MISMATCH",
            "config and probed device differ",
        ));
    }
    let config_digest = digest(c)?;
    let matrix_digest = digest(&matrix)?;
    let uuid = |role: &str| {
        Uuid::new_v5(
            &Uuid::NAMESPACE_OID,
            format!("{config_digest}:{}:{role}", disk.serial).as_bytes(),
        )
        .to_string()
    };
    let dev = disk.path.to_string_lossy().to_string();
    let suffix = if dev.chars().last().is_some_and(|c| c.is_ascii_digit()) {
        "p"
    } else {
        ""
    };
    let esp = format!("{dev}{suffix}1");
    let boot = format!("{dev}{suffix}2");
    let raw_root = format!("{dev}{suffix}3");
    let root_dev = if c.disk.encrypt {
        "/dev/mapper/cryptroot".into()
    } else {
        raw_root.clone()
    };
    let esp_size = size_mib(&c.disk.esp_size)?;
    let boot_size = size_mib(&c.disk.boot_size)?;
    if disk.size < (esp_size + boot_size + 8192) * 1024 * 1024 {
        return Err(BoreaError::invalid(
            "disk too small: partitions require at least 8 GiB for root",
        ));
    }
    let mut b = Builder { steps: Vec::new() };
    let mut partition = argv(&["sgdisk", "--zap-all", "--clear"]);
    for (n, size, kind, role) in [
        (1, format!("+{esp_size}M"), "ef00", "esp"),
        (2, format!("+{boot_size}M"), "8300", "boot"),
        (
            3,
            "0".into(),
            if c.disk.encrypt { "8309" } else { "8300" },
            "root",
        ),
    ] {
        partition.extend([
            format!("--new={n}:0:{size}"),
            format!("--typecode={n}:{kind}"),
            format!("--partition-guid={n}:{}", uuid(&format!("gpt.{role}"))),
        ]);
    }
    partition.push(dev.clone());
    b.cmd(
        "storage.partition",
        Phase::Storage,
        partition,
        false,
        Probe::None,
        Danger::Destructive,
    );
    b.cmd(
        "storage.settle",
        Phase::Storage,
        argv(&["udevadm", "settle"]),
        false,
        cmd_probe(&["test", "-b", &raw_root], false),
        Danger::Safe,
    );
    b.cmd(
        "storage.format.esp",
        Phase::Storage,
        argv(&[
            "mkfs.fat",
            "-F32",
            "-i",
            &uuid("fat").replace('-', "")[..8],
            &esp,
        ]),
        false,
        Probe::None,
        Danger::Destructive,
    );
    b.cmd(
        "storage.format.boot",
        Phase::Storage,
        argv(&["mkfs.ext4", "-F", "-U", &uuid("fs.boot"), &boot]),
        false,
        Probe::None,
        Danger::Destructive,
    );
    let mut secrets = vec![SecretSlot {
        name: "account.password".into(),
        file: c.account.password_file.clone(),
    }];
    if c.disk.encrypt {
        secrets.push(SecretSlot {
            name: "disk.luks".into(),
            file: c
                .disk
                .luks_password_file
                .clone()
                .unwrap_or(c.account.password_file.clone()),
        });
        b.add(
            "storage.luks.format",
            Phase::Storage,
            Action::Command {
                argv: argv(&[
                    "cryptsetup",
                    "luksFormat",
                    "--batch-mode",
                    "--type",
                    "luks2",
                    "--uuid",
                    &uuid("luks"),
                    "--key-file",
                    "-",
                    &raw_root,
                ]),
                chroot: false,
                stdin: Some("disk.luks".into()),
            },
            Probe::None,
            Danger::Destructive,
        );
        b.add(
            "storage.luks.open",
            Phase::Storage,
            Action::Command {
                argv: argv(&[
                    "cryptsetup",
                    "open",
                    "--key-file",
                    "-",
                    &raw_root,
                    "cryptroot",
                ]),
                chroot: false,
                stdin: Some("disk.luks".into()),
            },
            cmd_probe(&["test", "-b", "/dev/mapper/cryptroot"], false),
            Danger::Mutating,
        );
    }
    let mkfs = if c.disk.root_fs == Filesystem::Btrfs {
        "mkfs.btrfs"
    } else {
        "mkfs.ext4"
    };
    b.cmd(
        "storage.format.root",
        Phase::Storage,
        argv(&[mkfs, "-f", "-U", &uuid("fs.root"), &root_dev])
            .into_iter()
            .map(|s| {
                if mkfs == "mkfs.ext4" && s == "-f" {
                    "-F".into()
                } else {
                    s
                }
            })
            .collect(),
        false,
        Probe::None,
        Danger::Destructive,
    );
    b.mkdir("storage.root.dir", Phase::Storage, "/");
    if c.disk.root_fs == Filesystem::Btrfs {
        b.cmd(
            "storage.mount.top",
            Phase::Storage,
            argv(&["mount", "-o", "subvolid=5", &root_dev, "/mnt"]),
            false,
            cmd_probe(&["findmnt", "--mountpoint", "/mnt"], false),
            Danger::Mutating,
        );
        for sub in ["@", "@home", "@snapshots", "@log", "@cache"] {
            let path = format!("/mnt/{sub}");
            b.cmd(
                &format!("storage.subvolume.{}", sub.trim_start_matches('@')),
                Phase::Storage,
                argv(&["btrfs", "subvolume", "create", &path]),
                false,
                cmd_probe(&["btrfs", "subvolume", "show", &path], false),
                Danger::Mutating,
            );
        }
        b.cmd(
            "storage.unmount.top",
            Phase::Storage,
            argv(&["umount", "/mnt"]),
            false,
            cmd_probe(
                &["mountpoint", "--quiet", "/mnt/.borea-never-mounted"],
                false,
            ),
            Danger::Mutating,
        );
    }
    let options = if c.disk.root_fs == Filesystem::Btrfs {
        "subvol=@,compress=zstd:3,noatime,ssd,discard=async"
    } else {
        "defaults"
    };
    b.cmd(
        "storage.mount.root",
        Phase::Storage,
        argv(&["mount", "-o", options, &root_dev, "/mnt"]),
        false,
        cmd_probe(&["findmnt", "--mountpoint", "/mnt"], false),
        Danger::Mutating,
    );
    let mut fstab = String::new();
    if c.disk.root_fs == Filesystem::Btrfs {
        for (sub, path) in [
            ("@", "/"),
            ("@home", "/home"),
            ("@snapshots", "/.snapshots"),
            ("@log", "/var/log"),
            ("@cache", "/var/cache"),
        ] {
            fstab += &format!(
                "UUID={} {path} btrfs subvol={sub},compress=zstd:3,noatime,ssd,discard=async 0 0\n",
                uuid("fs.root")
            );
            if path != "/" {
                b.mkdir(
                    &format!("storage.dir.{}", sub.trim_start_matches('@')),
                    Phase::Storage,
                    path,
                );
                let mount = format!("/mnt{path}");
                b.cmd(
                    &format!("storage.mount.{}", sub.trim_start_matches('@')),
                    Phase::Storage,
                    argv(&[
                        "mount",
                        "-o",
                        &format!("subvol={sub},compress=zstd:3,noatime,ssd,discard=async"),
                        &root_dev,
                        &mount,
                    ]),
                    false,
                    cmd_probe(&["findmnt", "--mountpoint", &mount], false),
                    Danger::Mutating,
                );
            }
        }
    } else {
        fstab += &format!("UUID={} / ext4 defaults 0 1\n", uuid("fs.root"));
    }
    b.mkdir("storage.boot.dir", Phase::Storage, "/boot");
    b.cmd(
        "storage.mount.boot",
        Phase::Storage,
        argv(&["mount", &boot, "/mnt/boot"]),
        false,
        cmd_probe(&["findmnt", "--mountpoint", "/mnt/boot"], false),
        Danger::Mutating,
    );
    b.mkdir("storage.esp.dir", Phase::Storage, "/boot/efi");
    b.cmd(
        "storage.mount.esp",
        Phase::Storage,
        argv(&["mount", &esp, "/mnt/boot/efi"]),
        false,
        cmd_probe(&["findmnt", "--mountpoint", "/mnt/boot/efi"], false),
        Danger::Mutating,
    );
    fstab += &format!(
        "UUID={} /boot ext4 defaults 0 2\nPARTUUID={} /boot/efi vfat umask=0077 0 2\n",
        uuid("fs.boot"),
        uuid("gpt.esp")
    );
    let release = c.target.release.as_deref().unwrap_or(&family.release);
    let bootstrap = match c.target.family {
        FamilyId::Arch => argv(&["pacstrap", "-K", "/mnt"])
            .into_iter()
            .chain(family.base_packages.clone())
            .collect(),
        FamilyId::Debian => argv(&[
            "debootstrap",
            "--arch=amd64",
            release,
            "/mnt",
            "https://deb.debian.org/debian",
        ]),
        FamilyId::Fedora => argv(&[
            "dnf5",
            "-y",
            "--installroot=/mnt",
            &format!("--releasever={release}"),
            "install",
        ])
        .into_iter()
        .chain(family.base_packages.clone())
        .collect(),
        FamilyId::Suse => argv(&[
            "zypper",
            "--root",
            "/mnt",
            "--non-interactive",
            "install",
            "--no-recommends",
        ])
        .into_iter()
        .chain(family.base_packages.clone())
        .collect(),
    };
    b.cmd(
        &format!("bootstrap.{}", c.target.family.as_str()),
        Phase::Bootstrap,
        bootstrap,
        false,
        Probe::FileExists {
            path: "/etc/os-release".into(),
        },
        Danger::Mutating,
    );
    let mut packages = BTreeSet::new();
    let catalog = recipe::recipes()?;
    for id in &intent.components {
        let r = catalog.get(id).ok_or_else(|| {
            BoreaError::unsupported(format!("missing recipe {id}; choose a catalogue item"))
        })?;
        packages.extend(
            r.packages
                .get(&c.target.family)
                .ok_or_else(|| {
                    BoreaError::unsupported(format!(
                        "{id}: unpinned packaging path; omit until reviewed"
                    ))
                })?
                .iter()
                .cloned(),
        );
    }
    packages.extend(family.base_packages.clone());
    if c.disk.encrypt {
        packages.insert(
            match c.target.family {
                FamilyId::Debian => "cryptsetup-initramfs",
                _ => "cryptsetup",
            }
            .into(),
        );
    }
    if intent.greeter != "none" && intent.greeter != "greetd" {
        packages.insert(intent.greeter.clone());
    }
    let packages: Vec<_> = packages.into_iter().collect();
    b.cmd(
        "base.packages",
        Phase::Base,
        install(c.target.family, &packages),
        true,
        cmd_probe(&["test", "-e", "/var/lib/borea/base-packages"], true),
        Danger::Mutating,
    );
    b.file(
        "base.packages.receipt",
        Phase::Base,
        "/var/lib/borea/base-packages",
        packages.join("\n"),
        0o644,
    );
    b.file(
        "base.fstab",
        Phase::Base,
        "/etc/fstab",
        fstab.clone(),
        0o644,
    );
    if c.disk.encrypt {
        b.file(
            "base.crypttab",
            Phase::Base,
            "/etc/crypttab",
            format!("cryptroot UUID={} none luks\n", uuid("luks")),
            0o600,
        );
    }
    b.file(
        "base.hostname",
        Phase::Base,
        "/etc/hostname",
        format!("{}\n", c.target.hostname),
        0o644,
    );
    b.file(
        "base.hosts",
        Phase::Base,
        "/etc/hosts",
        format!(
            "127.0.0.1 localhost\n::1 localhost\n127.0.1.1 {}\n",
            c.target.hostname
        ),
        0o644,
    );
    b.file(
        "base.locale",
        Phase::Base,
        "/etc/locale.conf",
        format!("LANG={}\n", c.target.locale),
        0o644,
    );
    b.file(
        "base.keymap",
        Phase::Base,
        "/etc/vconsole.conf",
        format!("KEYMAP={}\n", c.target.keymap),
        0o644,
    );
    b.cmd(
        "base.timezone",
        Phase::Base,
        argv(&[
            "ln",
            "-sf",
            &format!("/usr/share/zoneinfo/{}", c.target.timezone),
            "/etc/localtime",
        ]),
        true,
        cmd_probe(
            &[
                "test",
                "/etc/localtime",
                "-ef",
                &format!("/usr/share/zoneinfo/{}", c.target.timezone),
            ],
            true,
        ),
        Danger::Mutating,
    );
    if matches!(c.target.family, FamilyId::Arch | FamilyId::Debian) {
        b.file(
            "base.locale.gen",
            Phase::Base,
            "/etc/locale.gen",
            format!("{} UTF-8\n", c.target.locale),
            0o644,
        );
        b.cmd(
            "base.locale.generate",
            Phase::Base,
            argv(&["locale-gen"]),
            true,
            cmd_probe(&["test", "-e", "/var/lib/borea/locale-generated"], true),
            Danger::Mutating,
        );
        b.file(
            "base.locale.receipt",
            Phase::Base,
            "/var/lib/borea/locale-generated",
            c.target.locale.clone(),
            0o644,
        );
    }
    b.cmd(
        "base.user",
        Phase::Base,
        argv(&[
            "useradd",
            "--create-home",
            "--shell",
            "/bin/bash",
            "--groups",
            &family.sudo_group,
            "--comment",
            &c.account.full_name,
            &c.account.username,
        ]),
        true,
        cmd_probe(&["id", "-u", &c.account.username], true),
        Danger::Mutating,
    );
    b.add(
        "base.password",
        Phase::Base,
        Action::Password {
            user: c.account.username.clone(),
            secret: "account.password".into(),
        },
        Probe::FileExists {
            path: "/var/lib/borea/password-set".into(),
        },
        Danger::Mutating,
    );
    b.file(
        "base.password.receipt",
        Phase::Base,
        "/var/lib/borea/password-set",
        config_digest.clone(),
        0o600,
    );
    b.file(
        "base.sudo",
        Phase::Base,
        "/etc/sudoers.d/10-borea",
        format!("%{} ALL=(ALL:ALL) ALL\n", family.sudo_group),
        0o440,
    );
    if c.disk.swap == "zram" {
        b.file(
            "base.zram",
            Phase::Base,
            "/etc/systemd/zram-generator.conf",
            "[zram0]\nzram-size = ram / 2\ncompression-algorithm = zstd\n".into(),
            0o644,
        );
    }
    for unit in [
        Some(family.network_unit.as_str()),
        c.account
            .ssh_authorized_key_file
            .as_ref()
            .map(|_| match c.target.family {
                FamilyId::Debian => "ssh.service",
                _ => "sshd.service",
            }),
    ]
    .into_iter()
    .flatten()
    {
        b.cmd(
            &format!("base.enable.{unit}"),
            Phase::Base,
            argv(&["systemctl", "enable", unit]),
            true,
            cmd_probe(&["systemctl", "is-enabled", unit], true),
            Danger::Mutating,
        );
    }
    if let Some(source) = &c.account.ssh_authorized_key_file {
        let path = PathBuf::from(format!("/home/{}/.ssh/authorized_keys", c.account.username));
        b.add(
            "base.ssh.key",
            Phase::Base,
            Action::CopyIn {
                source: source.clone(),
                dest: path.clone(),
                mode: 0o600,
            },
            Probe::FileExists { path },
            Danger::Mutating,
        );
        b.cmd(
            "base.ssh.permissions",
            Phase::Base,
            argv(&[
                "chown",
                "-R",
                &format!("{}:{}", c.account.username, c.account.username),
                &format!("/home/{}/.ssh", c.account.username),
            ]),
            true,
            cmd_probe(&["test", "-e", "/var/lib/borea/ssh-owned"], true),
            Danger::Mutating,
        );
        b.file(
            "base.ssh.receipt",
            Phase::Base,
            "/var/lib/borea/ssh-owned",
            "owned\n".into(),
            0o600,
        );
    }
    let mut checks = vec![
        Check::FileContains {
            path: "/etc/fstab".into(),
            needle: fstab,
        },
        Check::FileContains {
            path: "/etc/hostname".into(),
            needle: c.target.hostname.clone(),
        },
        Check::CommandOk {
            argv: argv(&["id", "-u", &c.account.username]),
        },
        Check::CommandOk {
            argv: argv(&["systemctl", "is-enabled", &family.network_unit]),
        },
    ];
    if let Some(session) = intent.session.filter(|_| c.session.kind != "omarchy") {
        let exec = session.exec.as_deref().unwrap_or("");
        if let Some(autostart) = intent.autostart {
            let dest = match c.session.compositor.as_deref() {
                Some("hyprland") => {
                    format!("/home/{}/.config/hypr/hyprland.conf", c.account.username)
                }
                _ => {
                    return Err(BoreaError::unsupported(
                        "shell autostart seeds currently implemented for Hyprland only; use compositor-only or Hyprland",
                    ));
                }
            };
            b.file("session.autostart",Phase::Session,&dest,format!("# Generated by borea\nmonitor = ,preferred,auto,1\nexec-once = {autostart}\nbind = SUPER, RETURN, exec, kitty\nbind = SUPER SHIFT, E, exit,\n"),0o644);
        }
        if intent.greeter == "greetd" {
            b.file("session.greetd",Phase::Session,"/etc/greetd/config.toml",format!("[terminal]\nvt = 1\n[default_session]\ncommand = \"tuigreet --cmd {exec}\"\nuser = \"greeter\"\n"),0o644);
        }
        if intent.greeter != "none" {
            let unit = format!("{}.service", intent.greeter);
            b.cmd(
                "session.greeter.enable",
                Phase::Session,
                argv(&["systemctl", "enable", &unit]),
                true,
                cmd_probe(&["systemctl", "is-enabled", &unit], true),
                Danger::Mutating,
            );
            checks.push(Check::CommandOk {
                argv: argv(&["systemctl", "is-enabled", &unit]),
            });
            b.cmd(
                "session.graphical",
                Phase::Session,
                argv(&["systemctl", "set-default", "graphical.target"]),
                true,
                cmd_probe(
                    &[
                        "test",
                        "/etc/systemd/system/default.target",
                        "-ef",
                        "/usr/lib/systemd/system/graphical.target",
                    ],
                    true,
                ),
                Danger::Mutating,
            );
        }
        if let Some(file) = session.session_file {
            checks.push(Check::FileExists {
                path: format!("/usr/share/wayland-sessions/{file}").into(),
            });
        }
    }
    let rootarg = if c.disk.encrypt {
        "root=/dev/mapper/cryptroot".to_string()
    } else {
        format!("root=UUID={}", uuid("fs.root"))
    };
    let subvol = if c.disk.root_fs == Filesystem::Btrfs {
        " rootflags=subvol=@"
    } else {
        ""
    };
    let cryptarg = if c.disk.encrypt {
        match c.target.family {
            FamilyId::Arch => format!(" cryptdevice=UUID={}:cryptroot", uuid("luks")),
            FamilyId::Fedora | FamilyId::Suse => format!(" rd.luks.uuid={}", uuid("luks")),
            _ => String::new(),
        }
    } else {
        String::new()
    };
    let cmdline = format!("{rootarg}{subvol}{cryptarg}");
    b.cmd(
        "boot.initramfs",
        Phase::Boot,
        family.initramfs.clone(),
        true,
        cmd_probe(&["test", "-e", "/var/lib/borea/initramfs-generated"], true),
        Danger::Mutating,
    );
    b.file(
        "boot.initramfs.receipt",
        Phase::Boot,
        "/var/lib/borea/initramfs-generated",
        cmdline.clone(),
        0o644,
    );
    if resolved == Loader::Limine {
        for (id, path) in [
            ("fallback", "/boot/efi/EFI/BOOT"),
            ("borea", "/boot/efi/EFI/borea"),
        ] {
            b.mkdir(&format!("boot.limine.dir.{id}"), Phase::Boot, path);
            b.cmd(
                &format!("boot.limine.copy.{id}"),
                Phase::Boot,
                argv(&[
                    "cp",
                    "/usr/share/limine/BOOTX64.EFI",
                    &format!("{path}/BOOTX64.EFI"),
                ]),
                true,
                cmd_probe(
                    &[
                        "cmp",
                        "-s",
                        "/usr/share/limine/BOOTX64.EFI",
                        &format!("{path}/BOOTX64.EFI"),
                    ],
                    true,
                ),
                Danger::Mutating,
            );
        }
        let contents = format!(
            "timeout: 5\nserial: yes\n\n/Borea\n    protocol: linux\n    path: guid({}):/vmlinuz-linux\n    module_path: guid({}):/initramfs-linux.img\n    cmdline: {cmdline}\n/Borea fallback\n    protocol: linux\n    path: guid({}):/vmlinuz-linux\n    module_path: guid({}):/initramfs-linux-fallback.img\n    cmdline: {cmdline}\n",
            uuid("fs.boot"),
            uuid("fs.boot"),
            uuid("fs.boot"),
            uuid("fs.boot")
        );
        b.file(
            "boot.limine.config",
            Phase::Boot,
            "/boot/efi/EFI/BOOT/limine.conf",
            contents.clone(),
            0o644,
        );
        b.file(
            "boot.limine.config.copy",
            Phase::Boot,
            "/boot/limine/limine.conf",
            contents,
            0o644,
        );
        checks.push(Check::FileExists {
            path: "/boot/efi/EFI/BOOT/BOOTX64.EFI".into(),
        });
    } else {
        b.file(
            "boot.grub.cmdline",
            Phase::Boot,
            "/etc/default/grub.d/90-borea.cfg",
            format!("GRUB_CMDLINE_LINUX=\"{cmdline}\"\n"),
            0o644,
        );
        b.cmd(
            "boot.grub.install",
            Phase::Boot,
            argv(&[
                &family.grub_install,
                "--target=x86_64-efi",
                "--efi-directory=/boot/efi",
                "--bootloader-id=borea",
                "--removable",
            ]),
            true,
            Probe::FileExists {
                path: "/boot/efi/EFI/BOOT/BOOTX64.EFI".into(),
            },
            Danger::Mutating,
        );
        let args = if c.target.family == FamilyId::Debian {
            vec![family.grub_mkconfig.clone()]
        } else {
            vec![
                family.grub_mkconfig.clone(),
                "-o".into(),
                family.grub_cfg.clone(),
            ]
        };
        b.cmd(
            "boot.grub.config",
            Phase::Boot,
            args,
            true,
            Probe::FileContains {
                path: family.grub_cfg.clone().into(),
                needle: uuid("fs.root"),
            },
            Danger::Mutating,
        );
        checks.push(Check::FileExists {
            path: family.grub_cfg.into(),
        });
    }
    if c.session.kind == "omarchy" {
        let selection = c
            .omarchy
            .as_ref()
            .cloned()
            .unwrap_or_default()
            .selection()?;
        // Do not let generic app requests contradict explicit keep/remove choices.
        for app in selection.removals() {
            for id in c
                .apps
                .enable
                .iter()
                .map(|id| format!("app.{id}"))
                .chain(selection.install_components.iter().cloned())
            {
                if catalog
                    .get(&id)
                    .and_then(|r| r.packages.get(&c.target.family))
                    .is_some_and(|ps| ps.iter().any(|pkg| app.packages.contains(pkg)))
                {
                    return Err(BoreaError::invalid(format!(
                        "{} is both removed and requested for installation",
                        app.id
                    )));
                }
            }
        }
        let user = &c.account.username;
        b.add(
            "omarchy.install",
            Phase::Finalise,
            Action::InstallOmarchy {
                user: user.clone(),
                source: selection.upstream.omarchy.clone(),
            },
            Probe::OmarchyInstalled {
                user: user.clone(),
                commit: selection.upstream.omarchy.commit.clone(),
            },
            Danger::Mutating,
        );
        // Replacements/browser additions are installed after upstream defaults and before removals.
        let additions = selection
            .install_components
            .iter()
            .flat_map(|id| {
                catalog[id]
                    .packages
                    .get(&c.target.family)
                    .into_iter()
                    .flatten()
                    .cloned()
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if !additions.is_empty() {
            b.cmd(
                "omarchy.apps.install",
                Phase::Finalise,
                install(c.target.family, &additions),
                true,
                Probe::CommandOk {
                    argv: argv(&["pacman", "-Q"])
                        .into_iter()
                        .chain(additions.iter().cloned())
                        .collect(),
                    chroot: true,
                },
                Danger::Mutating,
            );
        }
        for app in selection.removals() {
            if !app.packages.is_empty() {
                // Exact selected packages only: no dependency/orphan cascade and no --nodeps.
                let args = argv(&["pacman", "-R", "--noconfirm"])
                    .into_iter()
                    .chain(app.packages.clone())
                    .collect();
                b.cmd(
                    &format!("omarchy.apps.remove.{}", app.id),
                    Phase::Finalise,
                    args,
                    true,
                    Probe::PackagesAbsent {
                        packages: app.packages.clone(),
                    },
                    Danger::Mutating,
                );
                checks.push(Check::PackagesAbsent {
                    packages: app.packages.clone(),
                });
            } else if let Some(webapp) = &app.webapp {
                let helper = format!("/home/{user}/.local/share/omarchy/bin/omarchy-webapp-remove");
                let path = PathBuf::from(format!(
                    "/home/{user}/.local/share/applications/{webapp}.desktop"
                ));
                b.cmd(
                    &format!("omarchy.webapps.remove.{}", app.id),
                    Phase::Finalise,
                    argv(&["runuser", "-u", user, "--", &helper, webapp]),
                    true,
                    Probe::PathMissing { path: path.clone() },
                    Danger::Mutating,
                );
                checks.push(Check::PathMissing { path });
            }
        }
        if let Some(browser) = &selection.default_browser {
            let desktop = format!("{browser}.desktop");
            b.cmd(
                "omarchy.browser.default",
                Phase::Finalise,
                argv(&[
                    "runuser",
                    "-u",
                    user,
                    "--",
                    "xdg-settings",
                    "set",
                    "default-web-browser",
                    &desktop,
                ]),
                true,
                Probe::FileContains {
                    path: format!("/home/{user}/.config/mimeapps.list").into(),
                    needle: format!("x-scheme-handler/http={desktop}"),
                },
                Danger::Mutating,
            );
        }
        let selection_json = format!("{}\n", serde_json::to_string_pretty(&selection)?);
        b.file(
            "omarchy.selection",
            Phase::Finalise,
            "/var/lib/borea/omarchy-selection.json",
            selection_json,
            0o600,
        );
        b.file(
            "omarchy.defaults.policy",
            Phase::Finalise,
            "/var/lib/borea/defaults.json",
            format!(
                "{}\n",
                serde_json::to_string_pretty(&selection.policy(user)?)?
            ),
            0o600,
        );
        checks.push(Check::FileExists {
            path: format!("/home/{user}/.local/share/omarchy/install.sh").into(),
        });
        checks.push(Check::FileExists {
            path: "/usr/share/wayland-sessions/hyprland.desktop".into(),
        });
        checks.push(Check::CommandOk {
            argv: argv(&["systemctl", "is-enabled", "sddm.service"]),
        });
    }
    b.file(
        "finalise.receipt",
        Phase::Finalise,
        "/var/lib/borea/config-digest",
        config_digest.clone(),
        0o600,
    );
    let mut plan=Plan{schema:2,id:String::new(),created:"1970-01-01T00:00:00Z".into(),borea_version:version().into(),digest:String::new(),config_digest,matrix_digest,family:c.target.family,disk,root_fs:c.disk.root_fs,loader:resolved,root:"/mnt".into(),steps:b.steps,checks,secrets,notes:vec!["Experimental first implementation: no VM cell has passed. Live apply is gated pending storage probes, mount lifecycle and boot validation. Dry-run is available.".into()]};
    if c.session.kind == "omarchy" {
        plan.notes.push("Omarchy is an Arch source profile following pinned Linutil/upstream defaults. Only explicitly selected apps change; unlisted apps and Chromium are preserved. The user-context installer/independent runtime and VM cell are not yet validated; apply remains gated.".into());
    }
    plan.seal()?;
    Ok(plan)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn plan(fs: Filesystem, f: FamilyId) -> Plan {
        let mut c = Config::parse(include_str!("../../../recipes/profiles/minimal.toml")).unwrap();
        c.disk.root_fs = fs;
        c.target.family = f;
        c.apps.codecs = false;
        let disk = DeviceIdentity {
            path: c.disk.device.clone(),
            serial: "fixture-disk".into(),
            size: 20 * 1024 * 1024 * 1024,
            sector_size: 512,
        };
        build(
            &c,
            disk,
            Intent {
                components: vec![
                    "base.minimal".into(),
                    format!("fs.{}", fs.as_str()),
                    format!(
                        "boot.{}",
                        if f == FamilyId::Arch {
                            "limine"
                        } else {
                            "grub"
                        }
                    ),
                ],
                greeter: "none".into(),
                session: None,
                autostart: None,
            },
        )
        .unwrap()
    }
    #[test]
    fn schema_one_plans_require_replanning() {
        let mut p = plan(Filesystem::Btrfs, FamilyId::Arch);
        p.schema = 1;
        assert!(p.seal().is_err());
    }
    #[test]
    fn deterministic_all_families() {
        for f in FamilyId::ALL {
            for fs in [Filesystem::Btrfs, Filesystem::Ext4] {
                let p = plan(fs, f);
                assert_eq!(p.json().unwrap(), plan(fs, f).json().unwrap());
                p.validate().unwrap();
                assert!(p.steps.iter().any(|s| s.id == "base.fstab"));
            }
        }
    }
    #[test]
    fn limine_kernel_paths_use_partition_guid_syntax() {
        let p = plan(Filesystem::Ext4, FamilyId::Arch);
        let config = p
            .steps
            .iter()
            .find(|step| step.id == "boot.limine.config")
            .and_then(|step| match &step.action {
                Action::WriteFile { contents, .. } => Some(contents.as_str()),
                _ => None,
            })
            .unwrap();
        assert!(config.contains("serial: yes"));
        assert!(config.contains("path: guid(") && config.contains("):/vmlinuz-linux"));
        assert!(config.contains("module_path: guid(") && config.contains("):/initramfs-linux.img"));
    }
    #[test]
    fn digest_tampering_rejected() {
        let mut p = plan(Filesystem::Btrfs, FamilyId::Arch);
        p.steps[0].title.push('!');
        assert!(p.validate().is_err());
    }
    #[test]
    fn metadata_excluded_from_digest() {
        let mut p = plan(Filesystem::Btrfs, FamilyId::Arch);
        p.created = "2026-11-13T00:00:00Z".into();
        p.id = "metadata".into();
        p.validate().unwrap();
    }
    #[test]
    fn destructive_steps_only_storage() {
        let mut p = plan(Filesystem::Btrfs, FamilyId::Arch);
        p.steps.last_mut().unwrap().danger = Danger::Destructive;
        assert!(p.seal().is_err());
    }
}
