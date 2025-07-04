# Copyright (c) Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0

#!/usr/bin/env bash

# Utilize this script to generate a folder with random files,
# so we can test deployment of a site with multiple files

set -euo pipefail
shopt -s nullglob

# === GLOBAL VARIABLES ===
OUTPUT_DIR="./random_txt_files"
WORD_SOURCE="/usr/share/dict/words"
FILENAME_LENGTH=12
RANDOM_SUFFIX_DIGITS=6
TOTAL_FILES=0
PROGRESS_WIDTH=40

# === FUNCTION DEFINITIONS ===

parse_arguments() {
    if [[ $# -ne 1 ]] || [[ ! "$1" =~ ^[1-9][0-9]*$ ]]; then
        printf "Usage: %s <total_number_of_files>\n" "$0" >&2
        return 1
    fi
    TOTAL_FILES="$1"
}

generate_random_suffix() {
    local suffix;
    suffix=$(jot -r 1 100000 999999 2>/dev/null || printf "%06d" $((RANDOM % 900000 + 100000)))
    printf "%s" "$suffix"
}

generate_random_filename() {
    local base suffix filename;
    base=$(LC_CTYPE=C LC_ALL=C tr -dc 'a-zA-Z0-9' </dev/urandom | head -c "$FILENAME_LENGTH")
    if [[ -z "$base" ]] || [[ "$base" =~ [^a-zA-Z0-9] ]]; then
        printf "Error: Generated invalid base for filename\n" >&2
        return 1
    fi
    suffix=$(generate_random_suffix)
    filename="${base}_${suffix}.txt"
    printf "%s" "$filename"
}

get_random_word() {
    local word suffix;
    if ! word=$(shuf -n 1 "$WORD_SOURCE" 2>/dev/null); then
        if ! word=$(awk 'BEGIN {srand()} {lines[NR]=$0} END {print lines[int(rand()*NR)+1]}' "$WORD_SOURCE"); then
            printf "Error: Failed to extract random word from %s\n" "$WORD_SOURCE" >&2
            return 1
        fi
    fi
    if [[ -z "${word// /}" ]]; then
        printf "Warning: Extracted empty word, skipping\n" >&2
        return 1
    fi
    suffix=$(generate_random_suffix)
    printf "%s_%s" "$word" "$suffix"
}

create_random_file() {
    local filepath word;
    filepath="$1"
    if ! word=$(get_random_word); then
        return 1
    fi
    if ! printf "%s\n" "$word" > "$filepath"; then
        printf "Error: Failed to write to file '%s'\n" "$filepath" >&2
        return 1
    fi
}

prepare_output_directory() {
    mkdir -p "$OUTPUT_DIR"
}

draw_progress_bar() {
    local current=$1
    local total=$2
    local width=$PROGRESS_WIDTH
    local percent=$(( (current * 100 + total - 1) / total ))  # ceil percentage

    # ceil version of filled bar
    local filled=$(( (current * width + total - 1) / total ))
    local empty=$(( width - filled ))

    local bar_filled bar_empty bar;
    bar_filled=$(printf "%0.s#" $(seq 1 "$filled"))
    bar_empty=$(printf "%0.s-" $(seq 1 "$empty"))
    bar="${bar_filled}${bar_empty}"

    printf "\r[%s] %3d%% (%d/%d)" "$bar" "$percent" "$current" "$total"
}

generate_files() {
    local i=0 filename filepath;
    while [[ $i -lt $TOTAL_FILES ]]; do
        if ! filename=$(generate_random_filename); then
            continue
        fi
        filepath="$OUTPUT_DIR/$filename"
        if [[ -e "$filepath" ]]; then
            continue
        fi
        if create_random_file "$filepath"; then
            ((i++))
            draw_progress_bar "$i" "$TOTAL_FILES"
        fi
    done
    bar=$(printf "%0.s#" $(seq 1 "$PROGRESS_WIDTH"))
    printf "\r[%s] %3d%% (%d/%d)" "$bar" "100" "$i" "$TOTAL_FILES"
    printf "\n"
}

validate_word_source() {
    if [[ ! -f "$WORD_SOURCE" ]] || [[ ! -r "$WORD_SOURCE" ]]; then
        printf "Error: Word source file '%s' not found or not readable\n" "$WORD_SOURCE" >&2
        return 1
    fi
}

main() {
    parse_arguments "$@" || return 1
    validate_word_source || return 1
    prepare_output_directory
    generate_files
}

main "$@"
