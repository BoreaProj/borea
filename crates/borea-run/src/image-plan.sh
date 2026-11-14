# Closed driver for a sealed plan on a Borea disposable image (G05).
# Rust generates one program per probe/action under /plan and sends one
# operation name per line on stdin. This driver owns the mount namespace, the
# chroot lifecycle and teardown. It prints exactly one "rc N" line per operation.
set -u
root=$1
for tool in mount umount mountpoint chroot realpath awk install mkdir mv chmod dirname sync blkid sgdisk grep; do
    command -v "$tool" >/dev/null || { printf 'Missing runtime image tool: %s\n' "$tool" >&2; exit 1; }
done
exec 9<&0 </dev/null

# Resolve a plan path inside the target. Symlinks that leave the target are refused.
target() {
    p=$(realpath -m -- "$root$1") || return 1
    case "$p" in
        "$root"|"$root"/*) printf '%s\n' "$p" ;;
        *) printf 'Plan path leaves the target: %s\n' "$1" >&2; return 1 ;;
    esac
}
# contains FILE NEEDLE_FILE STRIP: literal multi-line substring test.
contains() {
    awk -v strip="$3" '
        FNR == NR { n = n $0 "\n"; next }
        { h = h $0 "\n" }
        END { if (strip) n = substr(n, 1, length(n) - 1); exit(index(h, n) ? 0 : 1) }
    ' "$2" "$1"
}
# Mount the API filesystems a chroot command needs. Idempotent.
chroot_ready() {
    test -x "$root/usr/bin/env" || { printf 'Target has no userland yet\n' >&2; return 1; }
    mkdir -p "$root/proc" "$root/sys" "$root/dev" "$root/run"
    mountpoint -q "$root/proc" || mount -t proc proc "$root/proc"
    mountpoint -q "$root/sys" || mount -t sysfs sys "$root/sys"
    mountpoint -q "$root/dev" || mount --bind /dev "$root/dev"
    mountpoint -q "$root/run" || mount -t tmpfs -o mode=0755 tmpfs "$root/run"
    # Package managers in the chroot need the sandbox resolver. Bind it; do not
    # copy it, so the target keeps its own file.
    if test -s /etc/resolv.conf && test -f "$root/etc/resolv.conf" \
        && ! test -L "$root/etc/resolv.conf" && ! mountpoint -q "$root/etc/resolv.conf"; then
        mount --bind /etc/resolv.conf "$root/etc/resolv.conf"
    fi
}
in_target() { chroot_ready && chroot "$root" "$@"; }
teardown() {
    sync
    mountpoint -q "$root" || return 0
    if ! umount -R "$root"; then
        printf 'Target is busy; detaching lazily after sync\n' >&2
        sync
        umount -R -l "$root"
    fi
}
trap 'exit 130' INT
trap 'exit 143' TERM HUP

while IFS= read -r op <&9; do
    secret=
    case "$op" in
        quit) break ;;
        s:*) IFS= read -r secret <&9 || exit 1; op=${op#s:} ;;
    esac
    case "$op" in
        ''|*[!a-z0-9]*) printf 'rc 125\n'; continue ;;
    esac
    ( . "/plan/$op" ) >&2
    printf 'rc %s\n' "$?"
    secret=
done
teardown >&2
printf 'rc %s\n' "$?"
