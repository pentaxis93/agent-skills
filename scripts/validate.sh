#!/usr/bin/env bash
set -euo pipefail

# validate.sh — Check SKILL.md files for correctness.
#
# Searches all configured source directories (from loadout.toml) for
# skills to validate.
#
# Usage:
#   ./scripts/validate.sh              # validate all skills in all sources
#   ./scripts/validate.sh skill-name   # validate one skill (searches sources)
#   ./scripts/validate.sh /path/to/dir # validate all skills in a specific dir

CONFIG="${LOADOUT_CONFIG:-${XDG_CONFIG_HOME:-$HOME/.config}/loadout/loadout.toml}"

errors=0
checked=0

# ── Resolve source directories ──────────────────────────────────────────

get_source_dirs() {
  if [ -f "$CONFIG" ]; then
    python3 -c "
import sys, json, os
try:
    import tomllib
except ImportError:
    import tomli as tomllib

with open(os.path.expanduser('$CONFIG'), 'rb') as f:
    config = tomllib.load(f)
for s in config.get('sources', {}).get('skills', []):
    print(os.path.expanduser(s))
" 2>/dev/null
  fi
}

# ── Validation ──────────────────────────────────────────────────────────

validate_one() {
  local skill_dir="$1"
  local dir_name="$(basename "$skill_dir")"
  local skill_file="$skill_dir/SKILL.md"

  if [ ! -f "$skill_file" ]; then
    echo "FAIL [$dir_name]: SKILL.md not found at $skill_file"
    errors=$((errors + 1))
    return
  fi

  checked=$((checked + 1))

  python3 -c "
import sys, re

with open('$skill_file') as f:
    content = f.read()

if not content.startswith('---'):
    print('FAIL [$dir_name]: no YAML frontmatter (must start with ---)')
    sys.exit(1)

parts = content.split('---', 2)
if len(parts) < 3:
    print('FAIL [$dir_name]: malformed frontmatter (missing closing ---)')
    sys.exit(1)

try:
    import yaml
except ImportError:
    fm = {}
    for line in parts[1].strip().split('\n'):
        line = line.strip()
        if ':' in line and not line.startswith('#'):
            key, _, val = line.partition(':')
            fm[key.strip()] = val.strip().strip('\"').strip(\"'\")
else:
    fm = yaml.safe_load(parts[1]) or {}

errors = 0

if 'name' not in fm:
    print('FAIL [$dir_name]: missing required field: name')
    errors += 1

if 'description' not in fm:
    print('FAIL [$dir_name]: missing required field: description')
    errors += 1

if errors:
    sys.exit(1)

name = str(fm['name'])
desc = str(fm['description'])

name_re = r'^[a-z0-9]+(-[a-z0-9]+)*$'
if not re.match(name_re, name):
    print(f'FAIL [$dir_name]: name \"{name}\" does not match {name_re}')
    errors += 1

if len(name) > 64:
    print(f'FAIL [$dir_name]: name exceeds 64 characters ({len(name)})')
    errors += 1

if name != '$dir_name':
    print(f'FAIL [$dir_name]: name \"{name}\" does not match directory \"$dir_name\"')
    errors += 1

if len(desc) > 1024:
    print(f'FAIL [$dir_name]: description exceeds 1024 characters ({len(desc)})')
    errors += 1

if len(desc) == 0:
    print(f'FAIL [$dir_name]: description is empty')
    errors += 1

if errors:
    sys.exit(1)

print(f'  OK  [{name}]: \"{desc[:60]}{\"...\" if len(desc) > 60 else \"\"}\"')
" || errors=$((errors + 1))
}

# ── Main ────────────────────────────────────────────────────────────────

if [ $# -gt 0 ] && [ -d "$1" ]; then
  # Validate all skills in a specific directory
  target_dir="$1"
  echo "Validating skills in $target_dir..."
  echo ""
  for skill_dir in "$target_dir"/*/; do
    [ -d "$skill_dir" ] || continue
    validate_one "${skill_dir%/}"
  done
elif [ $# -gt 0 ]; then
  # Validate a specific skill by name — search sources
  name="$1"
  found=false
  while IFS= read -r src_dir; do
    if [ -d "$src_dir/$name" ]; then
      echo "Validating $name in $src_dir..."
      echo ""
      validate_one "$src_dir/$name"
      found=true
      break
    fi
  done < <(get_source_dirs)
  if [ "$found" = false ]; then
    echo "error: skill '$name' not found in any source directory" >&2
    exit 1
  fi
else
  # Validate all skills across all source directories
  while IFS= read -r src_dir; do
    if [ ! -d "$src_dir" ]; then continue; fi
    echo "Validating skills in $src_dir..."
    echo ""
    for skill_dir in "$src_dir"/*/; do
      [ -d "$skill_dir" ] || continue
      validate_one "${skill_dir%/}"
    done
  done < <(get_source_dirs)
fi

echo ""
if [ $errors -gt 0 ]; then
  echo "FAILED: $errors error(s) in $checked skill(s)"
  exit 1
else
  echo "PASSED: $checked skill(s) validated"
fi
