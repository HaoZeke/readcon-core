#!/bin/bash
# Run a command in a private mount and UTS namespace.
# /etc/hostname in that namespace reads bench. A command that is already
# there runs as itself.

set -eu

if [ "${1:-}" = "--inside" ]; then
  shift
  mount --make-rprivate /
  printf '%s\n' bench > /tmp/bench-hostname
  mount --bind /tmp/bench-hostname /etc/hostname
  hostname bench
  exec "$@"
fi

export READCON_HOST_UTS="$(readlink /proc/self/ns/uts)"
export READCON_HOST_MNT="$(readlink /proc/self/ns/mnt)"
exec unshare --user --map-root-user --mount --uts --fork "$0" --inside "$@"
