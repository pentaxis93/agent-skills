#!/usr/bin/env bash
set -euo pipefail

# install.sh — Link enabled skills into tool discovery paths.
#
# Reads ~/.config/loadout/loadout.toml (or $LOADOUT_CONFIG) to determine
# which skills to enable and where. Skills are resolved from configured
# source directories.
#
# Usage:
#   ./scripts/install.sh              # apply loadout.toml
#   ./scripts/install.sh --dry-run    # show what would happen
#   ./scripts/install.sh --clean      # remove all managed symlinks
#   ./scripts/install.sh --list       # list enabled skills per scope

CONFIG="${LOADOUT_CONFIG:-${XDG_CONFIG_HOME:-$HOME/.config}/loadout/loadout.toml}"
MARKER=".managed-by-loadout"
DRY_RUN=false
CLEAN=false
LIST=false

# ── Parse args ──────────────────────────────────────────────────────────

for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN=true ;;
    --clean)   CLEAN=true ;;
    --list)    LIST=true ;;
    --help|-h)
      echo "Usage: install.sh [--dry-run] [--clean] [--list]"
      echo ""
      echo "  --dry-run  Show what would happen without making changes"
      echo "  --clean    Remove all managed symlinks from target directories"
      echo "  --list     List enabled skills per scope and exit"
      echo ""
      echo "Config: \$LOADOUT_CONFIG or ~/.config/loadout/loadout.toml"
      exit 0
      ;;
    *)
      echo "Unknown option: $arg" >&2
      exit 1
      ;;
  esac
done

# ── Dependency check ────────────────────────────────────────────────────

if ! command -v python3 &>/dev/null; then
  echo "error: python3 is required to parse TOML" >&2
  exit 1
fi

if [ ! -f "$CONFIG" ]; then
  echo "error: config not found: $CONFIG" >&2
  echo ""
  echo "Create it with:"
  echo "  mkdir -p ~/.config/loadout"
  echo "  cp loadout.example.toml ~/.config/loadout/loadout.toml"
  exit 1
fi

# ── TOML parser ─────────────────────────────────────────────────────────

parse_toml() {
  python3 -c "
import sys, json, os
try:
    import tomllib
except ImportError:
    try:
        import tomli as tomllib
    except ImportError:
        print('error: Python 3.11+ or tomli package required', file=sys.stderr)
        sys.exit(1)

with open(os.path.expanduser('$CONFIG'), 'rb') as f:
    config = tomllib.load(f)
print(json.dumps(config))
"
}

CONFIG_JSON="$(parse_toml)"

# ── Extract config ──────────────────────────────────────────────────────

expand_path() {
  echo "${1/#\~/$HOME}"
}

source_dirs() {
  echo "$CONFIG_JSON" | python3 -c "
import sys, json, os
config = json.load(sys.stdin)
for s in config.get('sources', {}).get('skills', []):
    print(os.path.expanduser(s))
"
}

global_targets() {
  echo "$CONFIG_JSON" | python3 -c "
import sys, json
config = json.load(sys.stdin)
for t in config.get('global', {}).get('targets', []):
    print(t)
"
}

global_skills() {
  echo "$CONFIG_JSON" | python3 -c "
import sys, json
config = json.load(sys.stdin)
for s in config.get('global', {}).get('skills', []):
    print(s)
"
}

project_entries() {
  echo "$CONFIG_JSON" | python3 -c "
import sys, json
config = json.load(sys.stdin)
projects = config.get('projects', {})
for path, cfg in projects.items():
    inherit = cfg.get('inherit', True)
    skills = ','.join(cfg.get('skills', []))
    print(f'{path}|{inherit}|{skills}')
"
}

# ── Skill resolution ────────────────────────────────────────────────────
# Search configured source directories for a skill by name.
# Returns the first match (sources are searched in order).

resolve_skill() {
  local name="$1"
  while IFS= read -r src_dir; do
    local candidate="$src_dir/$name"
    if [ -d "$candidate" ] && [ -f "$candidate/SKILL.md" ]; then
      echo "$candidate"
      return 0
    fi
  done < <(source_dirs)
  return 1
}

# ── Validation ──────────────────────────────────────────────────────────

validate_name() {
  local name="$1"
  if [[ ! "$name" =~ ^[a-z0-9]+(-[a-z0-9]+)*$ ]]; then
    echo "error: invalid skill name '$name'" >&2
    return 1
  fi
  if [ ${#name} -gt 64 ]; then
    echo "error: skill name '$name' exceeds 64 characters" >&2
    return 1
  fi
}

validate_skill() {
  local name="$1"
  validate_name "$name" || return 1
  if ! resolve_skill "$name" >/dev/null; then
    echo "error: skill '$name' not found in any source directory" >&2
    echo "  searched:" >&2
    source_dirs | while read -r d; do echo "    $d" >&2; done
    return 1
  fi
}

# ── Actions ─────────────────────────────────────────────────────────────

place_marker() {
  local target_dir="$1"
  local marker_file="$target_dir/$MARKER"
  if [ "$DRY_RUN" = true ]; then
    echo "  [dry-run] would create marker: $marker_file"
  else
    mkdir -p "$target_dir"
    echo "loadout" > "$marker_file"
  fi
}

link_skill() {
  local name="$1"
  local target_dir="$2"
  local dest="$(expand_path "$target_dir")/$name"
  local src
  src="$(resolve_skill "$name")" || return 1

  if [ "$DRY_RUN" = true ]; then
    echo "  [dry-run] $src -> $dest"
    return
  fi

  mkdir -p "$(expand_path "$target_dir")"

  # Remove existing (managed) symlink or directory
  if [ -L "$dest" ]; then
    rm "$dest"
  elif [ -d "$dest" ]; then
    local parent_marker="$(expand_path "$target_dir")/$MARKER"
    if [ -f "$parent_marker" ]; then
      rm -rf "$dest"
    else
      echo "  warning: $dest exists and is not managed, skipping" >&2
      return
    fi
  fi

  ln -s "$src" "$dest"
  echo "  linked: $name -> $dest"
}

clean_target() {
  local target_dir="$(expand_path "$1")"
  local marker_file="$target_dir/$MARKER"

  if [ ! -f "$marker_file" ]; then
    return
  fi

  if [ "$DRY_RUN" = true ]; then
    echo "  [dry-run] would clean managed symlinks in $target_dir"
    return
  fi

  # Remove all symlinks in managed directory
  for entry in "$target_dir"/*/; do
    entry="${entry%/}"
    if [ -L "$entry" ]; then
      rm "$entry"
      echo "  removed: $entry"
    fi
  done

  rm "$marker_file"
  echo "  removed marker: $marker_file"

  # Remove directory if empty
  if [ -d "$target_dir" ] && [ -z "$(ls -A "$target_dir")" ]; then
    rmdir "$target_dir"
    echo "  removed empty: $target_dir"
  fi
}

# ── List mode ───────────────────────────────────────────────────────────

if [ "$LIST" = true ]; then
  echo "=== Sources ==="
  source_dirs | while read -r d; do
    count=0
    if [ -d "$d" ]; then
      count=$(find "$d" -maxdepth 2 -name SKILL.md 2>/dev/null | wc -l)
    fi
    echo "  $d ($count skills)"
  done

  echo ""
  echo "=== Global skills ==="
  echo "Targets:"
  global_targets | while read -r t; do
    echo "  $(expand_path "$t")"
  done
  echo "Skills:"
  has_skills=false
  global_skills | while read -r s; do
    if [ -n "$s" ]; then
      has_skills=true
      local_src="$(resolve_skill "$s" 2>/dev/null || echo "(not found)")"
      echo "  $s  <- $local_src"
    fi
  done
  if [ "$has_skills" = false ]; then
    echo "  (none)"
  fi

  echo ""
  echo "=== Project skills ==="
  has_projects=false
  project_entries | while IFS='|' read -r path inherit skills; do
    if [ -n "$path" ]; then
      has_projects=true
      echo "  $path (inherit=$inherit): $skills"
    fi
  done
  if [ "$has_projects" = false ]; then
    echo "  (none configured)"
  fi
  exit 0
fi

# ── Clean mode ──────────────────────────────────────────────────────────

if [ "$CLEAN" = true ]; then
  echo "Cleaning managed symlinks..."
  global_targets | while read -r t; do
    clean_target "$t"
  done
  project_entries | while IFS='|' read -r path inherit skills; do
    if [ -n "$path" ]; then
      clean_target "$path/.claude/skills"
      clean_target "$path/.opencode/skills"
      clean_target "$path/.agents/skills"
    fi
  done
  echo "Done."
  exit 0
fi

# ── Install mode ────────────────────────────────────────────────────────

echo "Config: $CONFIG"
echo "Sources:"
source_dirs | while read -r d; do echo "  $d"; done
echo ""

# Validate all skills first
fail=false
global_skills | while read -r s; do
  if [ -n "$s" ]; then
    validate_skill "$s" || fail=true
  fi
done

project_entries | while IFS='|' read -r path inherit skills; do
  IFS=',' read -ra skill_array <<< "$skills"
  for s in "${skill_array[@]}"; do
    if [ -n "$s" ]; then
      validate_skill "$s" || fail=true
    fi
  done
done

# Link global skills
echo "--- Global scope ---"
global_targets | while read -r target; do
  expanded="$(expand_path "$target")"
  echo "Target: $expanded"
  mkdir -p "$expanded"
  place_marker "$expanded"
  global_skills | while read -r s; do
    if [ -n "$s" ]; then
      link_skill "$s" "$target"
    fi
  done
done

# Link project skills
project_entries | while IFS='|' read -r path inherit skills; do
  if [ -z "$path" ]; then continue; fi

  echo ""
  echo "--- Project: $path ---"

  for subdir in ".claude/skills" ".opencode/skills" ".agents/skills"; do
    project_target="$path/$subdir"
    expanded="$(expand_path "$project_target")"
    echo "Target: $expanded"
    mkdir -p "$expanded"
    place_marker "$expanded"

    if [ "$inherit" = "True" ] || [ "$inherit" = "true" ]; then
      global_skills | while read -r gs; do
        if [ -n "$gs" ]; then
          link_skill "$gs" "$project_target"
        fi
      done
    fi

    IFS=',' read -ra skill_array <<< "$skills"
    for s in "${skill_array[@]}"; do
      if [ -n "$s" ]; then
        link_skill "$s" "$project_target"
      fi
    done
  done
done

echo ""
echo "Done."
