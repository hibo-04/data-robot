#!/bin/sh
# Host has no gcc. rustc may pass `-flavor gnu`; ld.lld already is the GNU driver
# and rejects that flag, so drop it.
set -e
LLD="/home/johannes-haller/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/gcc-ld/ld.lld"
LIBGCC="/home/johannes-haller/Dokumente/Projekte/data-robot/tools/linker-lib"

if [ "$1" = "-flavor" ]; then
    shift 2
fi

shared=0
for arg in "$@"; do
    case "$arg" in
        -shared|--shared) shared=1 ;;
    esac
done

if [ "$shared" -eq 1 ]; then
    exec "$LLD" \
        -L/usr/lib/x86_64-linux-gnu \
        -L/lib/x86_64-linux-gnu \
        -L"$LIBGCC" \
        "$@"
fi

exec "$LLD" \
    -L/usr/lib/x86_64-linux-gnu \
    -L/lib/x86_64-linux-gnu \
    -L"$LIBGCC" \
    /usr/lib/x86_64-linux-gnu/Scrt1.o \
    /usr/lib/x86_64-linux-gnu/crti.o \
    "$@" \
    /usr/lib/x86_64-linux-gnu/crtn.o \
    --dynamic-linker=/lib64/ld-linux-x86-64.so.2
