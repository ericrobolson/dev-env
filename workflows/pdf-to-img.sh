#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat >&2 <<'EOF'
Usage: pdf-to-img.sh <name> <pdf-path>

Convert each PDF page to a 150 DPI JPEG in .tmp/<timestamp>_<name>/.
Example: pdf-to-img.sh invoice ./invoice.pdf
EOF
}

fail() {
    printf 'Error: %s\n' "$1" >&2
    exit 1
}

if [[ $# -ne 2 ]]; then
    usage
    exit 2
fi

name="$1"
pdf_path="$2"

[[ -n "$name" ]] || fail 'name must not be empty'
[[ -f "$pdf_path" && -r "$pdf_path" ]] || fail "PDF file does not exist or is not readable: $pdf_path"
[[ "${pdf_path##*.}" == [pP][dD][fF] ]] || fail "input file must have a .pdf extension: $pdf_path"
command -v pdftoppm >/dev/null 2>&1 || fail 'pdftoppm is required (install Poppler)'

# Make relative paths absolute so paths beginning with a dash are not parsed as options.
[[ "$pdf_path" == /* ]] || pdf_path="$PWD/$pdf_path"

safe_name=$(printf '%s' "$name" | LC_ALL=C tr -cs '[:alnum:]_.-' '_' | sed -E 's/^[_\.]+//; s/[_\.]+$//')
[[ -n "$safe_name" ]] || fail 'name must contain at least one letter or number'

mkdir -p .tmp
timestamp=$(date '+%y%m%d-%H%M%S')
output_dir=".tmp/${timestamp}_${safe_name}"
[[ ! -e "$output_dir" ]] || fail "output directory already exists: $output_dir"
mkdir "$output_dir"

if ! pdftoppm -jpeg -r 150 "$pdf_path" "$output_dir/page"; then
    fail "PDF conversion failed; partial output is in $output_dir"
fi

page=1
while [[ -f "$output_dir/page-$page.jpg" ]]; do
    printf -v numbered '%03d.jpg' "$((page - 1))"
    mv "$output_dir/page-$page.jpg" "$output_dir/$numbered"
    page=$((page + 1))
done

(( page > 1 )) || fail "conversion produced no pages; inspect the PDF and partial output in $output_dir"
printf 'Converted %d page(s) to %s\n' "$((page - 1))" "$output_dir"
