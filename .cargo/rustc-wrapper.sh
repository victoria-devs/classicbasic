#!/bin/bash
RUSTC="$1"
shift
ARGS=()
PREV=""
for arg in "$@"; do
    if [ "$PREV" = "--target" ]; then
        ARGS+=("${arg//alpine/unknown}")
    elif [[ "$arg" == --target=*alpine* ]]; then
        ARGS+=("${arg//alpine/unknown}")
    else
        ARGS+=("$arg")
    fi
    PREV="$arg"
done
exec "$RUSTC" "${ARGS[@]}"
