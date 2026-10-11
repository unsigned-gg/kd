#!/usr/bin/env bash
# herdr-workspace.sh: open a repo as an estate agent workspace in herdr.
#
# Usage: scripts/herdr-workspace.sh [DIR] [LABEL]
#   DIR    the repo checkout (default: the current directory)
#   LABEL  the workspace label (default: the directory name)
#
# Layout (measured on cerebral-work/cas-substrate, 2026-10-08):
#   tab "agents": Claude Code on Fable | Antigravity
#   tab "saga":   dagr  | skald | edda      (top row)
#                 gates | runs  | world     (bottom row)
#   tab "omp":    omp-1 | omp-2 | omp-3
#
# Environment:
#   FABLE_MODEL  Claude model for the agents tab (default claude-fable-5-1)
#   OMP_WORKERS  number of omp panes, 1 to 3 (default 3)
#   NO_AGENTS=1  build the layout and the saga panes, but start no agents
#
# Run it from inside a herdr pane. It never moves your focus. Claude Code asks
# to trust a folder the first time; that prompt is yours to answer.
set -euo pipefail

die() { echo "herdr-workspace: $*" >&2; exit 1; }

[ "${HERDR_ENV:-}" = 1 ] || die "run this inside a herdr pane (HERDR_ENV=1)"
command -v herdr >/dev/null || die "herdr is not on PATH"
command -v jq >/dev/null || die "jq is not on PATH"

dir=$(cd "${1:-.}" && pwd)
label=${2:-$(basename "$dir")}
model=${FABLE_MODEL:-claude-fable-5-1}
workers=${OMP_WORKERS:-3}
case "$workers" in 1|2|3) ;; *) die "OMP_WORKERS must be 1, 2 or 3" ;; esac

# Agent names must match [a-z][a-z0-9_-]{0,31} and be unique among live agents.
slug=$(printf '%s' "$label" | tr '[:upper:]' '[:lower:]' | tr -c 'a-z0-9_-' '-' | sed 's/^[^a-z]*//' | cut -c1-20)
[ -n "$slug" ] || slug=ws

# The saga edda pane follows the project named by the origin remote (org/repo).
project=$(git -C "$dir" remote get-url origin 2>/dev/null | sed -E 's#^.*[:/]([^/]+/[^/]+)$#\1#; s#\.git$##' || true)

missing=()
for bin in claude agy omp saga dagr skald; do
  command -v "$bin" >/dev/null || missing+=("$bin")
done
[ ${#missing[@]} -eq 0 ] || echo "herdr-workspace: not on PATH, those panes will fail: ${missing[*]}" >&2

split() { # split PANE DIRECTION RATIO -> new pane id
  herdr pane split "$1" --direction "$2" --ratio "$3" --cwd "$dir" --no-focus | jq -er '.result.pane.pane_id'
}
name() { herdr pane rename "$1" "$2" >/dev/null; }
run() { # run PANE COMMAND...: herdr types the words into the pane's shell
  local pane=$1; shift
  herdr pane run "$pane" "$@" >/dev/null
}
start() { # start NAME KIND PANE [-- ARGS]
  local out
  if out=$(herdr agent start "$@" --timeout 90000 2>&1) && jq -e '.result' >/dev/null <<<"$out"; then
    echo "  started $1 in $3"
  else
    echo "  $1 in $3 is not ready: $(jq -r '.error.message // empty' <<<"$out" 2>/dev/null || echo "$out")" >&2
  fi
}

ws_json=$(herdr workspace create --cwd "$dir" --label "$label" --no-focus)
ws=$(jq -er '.result.workspace.workspace_id' <<<"$ws_json")
agents_tab=$(jq -er '.result.tab.tab_id' <<<"$ws_json")
fable=$(jq -er '.result.root_pane.pane_id' <<<"$ws_json")
echo "workspace $ws ($label) at $dir"

# Tab "agents": Claude Code on Fable | Antigravity.
herdr tab rename "$agents_tab" agents >/dev/null
agy_pane=$(split "$fable" right 0.5)
name "$fable" fable
name "$agy_pane" antigravity

# Tab "saga": three columns, two rows.
saga_json=$(herdr tab create --workspace "$ws" --cwd "$dir" --label saga --no-focus)
dagr_pane=$(jq -er '.result.root_pane.pane_id' <<<"$saga_json")
skald_pane=$(split "$dagr_pane" right 0.333)
edda_pane=$(split "$skald_pane" right 0.5)
gates_pane=$(split "$dagr_pane" down 0.5)
runs_pane=$(split "$skald_pane" down 0.5)
world_pane=$(split "$edda_pane" down 0.5)
for p in "$dagr_pane:dagr" "$skald_pane:skald" "$edda_pane:edda" \
         "$gates_pane:gates" "$runs_pane:runs" "$world_pane:world"; do
  name "${p%:*}" "${p##*:}"
done
run "$dagr_pane" dagr view
run "$skald_pane" skald view
if [ -n "$project" ]; then
  run "$edda_pane" saga edda --watch --project "$project"
else
  run "$edda_pane" saga edda --watch
fi
run "$gates_pane" saga gates
run "$runs_pane" saga runs --watch
run "$world_pane" saga world --watch

# Tab "omp": up to three workers in equal columns.
omp_json=$(herdr tab create --workspace "$ws" --cwd "$dir" --label omp --no-focus)
omp_panes=("$(jq -er '.result.root_pane.pane_id' <<<"$omp_json")")
if [ "$workers" -ge 2 ]; then
  ratio=0.5; [ "$workers" -eq 3 ] && ratio=0.333
  omp_panes+=("$(split "${omp_panes[0]}" right "$ratio")")
fi
[ "$workers" -eq 3 ] && omp_panes+=("$(split "${omp_panes[1]}" right 0.5)")
for i in "${!omp_panes[@]}"; do name "${omp_panes[$i]}" "omp-$((i + 1))"; done

if [ "${NO_AGENTS:-}" = 1 ]; then
  echo "NO_AGENTS=1: layout built, no agents started"
else
  start "$slug-fable" --kind claude --pane "$fable" -- --model "$model"
  start "$slug-agy" --kind agy --pane "$agy_pane"
  for i in "${!omp_panes[@]}"; do
    start "$slug-omp-$((i + 1))" --kind omp --pane "${omp_panes[$i]}"
  done
fi
echo "done: herdr workspace focus $ws"
