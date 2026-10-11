# Makefile for rust-monorepo-template
# Standard POSIX wrapper that awk-parses recipes directly from Justfile.
SHELL := bash
.SHELLFLAGS := -euo pipefail -c
.DEFAULT_GOAL := help
.ONESHELL:

JOBS ?= 2
NEXTEST_PROFILE ?= default
ORG ?= cerebral-work
NAME ?=
REPO ?=
VERSION ?=

export JOBS NEXTEST_PROFILE ORG NAME REPO VERSION

JUST := $(shell command -v just 2>/dev/null)

# Discover available recipe names from Justfile (excluding help, default, and template targets)
TARGETS := $(shell awk '(/^[a-zA-Z0-9_-]+ *:[^=]/ || /^[a-zA-Z0-9_-]+ *:$$/) && !/^[a-zA-Z0-9_-]+ *:=/ { split($$1, a, ":"); t = a[1]; if (t != "help" && t != "default" && t != "new-repo" && t != "template-check") print t }' Justfile 2>/dev/null)

.PHONY: $(TARGETS) help default

define AWK_PARSER
BEGIN {
    vars["jobs"] = jobs
    vars["nextest_profile"] = nextest_profile
    vars["org"] = org
}
# Capture variable definitions: var := "val"
/^[a-zA-Z0-9_-]+ *:=/ {
    var = $$1
    val = $$0
    sub(/^[a-zA-Z0-9_-]+ *:= *"/, "", val)
    sub(/".*$$/, "", val)
    if (!(var in vars)) vars[var] = val
    next
}
# Capture doc comment above recipe
/^# / {
    sub(/^# */, "")
    last_doc = $$0
    next
}
# Capture recipe signature
(/^[a-zA-Z0-9_-]+ *:[^=]/ || /^[a-zA-Z0-9_-]+ *:$$/) && !/^[a-zA-Z0-9_-]+ *:=/ {
    curr = $$1
    sub(/:.*/, "", curr)
    header = $$0
    sub(/^[a-zA-Z0-9_-]+ *: */, "", header)
    deps[curr] = header
    all_targets[curr] = 1
    doc[curr] = last_doc
    order[++target_count] = curr
    last_doc = ""
    next
}
# Capture recipe body lines and interpolate variables
/^[ \t]+/ && curr != "" {
    line = $$0
    sub(/^[ \t]+/, "", line)
    for (v in vars) {
        gsub("\\{\\{ *" v " *\\}\\}", vars[v], line)
    }
    gsub(/[$$][$$]/, "$$", line)
    body[curr] = (body[curr] ? body[curr] "\n" : "") line
    next
}
!/^[ \t]/ {
    curr = ""
    last_doc = ""
}
function emit(t,    n, d, i, k, lines, cmd) {
    if (!(t in all_targets)) {
        print "echo \"error: unknown recipe: " t "\" >&2; exit 1"
        return
    }
    if (deps[t] != "") {
        n = split(deps[t], d, " ")
        for (i = 1; i <= n; i++) {
            if (d[i] != "") emit(d[i])
        }
    }
    if (body[t] != "") {
        k = split(body[t], lines, "\n")
        for (i = 1; i <= k; i++) {
            cmd = lines[i]
            sub(/^@/, "", cmd)
            print cmd
        }
    }
}
function show_help() {
    print "Usage: make <target>"
    print ""
    print "Available recipes (parsed from Justfile):"
    for (i = 1; i <= target_count; i++) {
        t = order[i]
        if (t != "default" && t != "help") {
            printf "  \033[36m%-24s\033[0m %s\n", t, (doc[t] ? doc[t] : "")
        }
    }
}
END {
    if (target == "help" || target == "default" || target == "") {
        show_help()
    } else {
        print "set -euo pipefail"
        emit(target)
    }
}
endef
export AWK_PARSER

help:
	@if [ -n "$(JUST)" ]; then \
		$(JUST) --list; \
	else \
		awk -v target="help" -v jobs="$(JOBS)" -v nextest_profile="$(NEXTEST_PROFILE)" -v org="$(ORG)" "$$AWK_PARSER" Justfile; \
	fi

default: help

$(TARGETS):
	@if [ -n "$(JUST)" ]; then \
		$(JUST) --set jobs "$(JOBS)" "$@"; \
	else \
		awk -v target="$@" -v jobs="$(JOBS)" -v nextest_profile="$(NEXTEST_PROFILE)" -v org="$(ORG)" "$$AWK_PARSER" Justfile | $(SHELL); \
	fi

%:
	@if [ -n "$(JUST)" ]; then \
		$(JUST) --set jobs "$(JOBS)" "$*"; \
	else \
		awk -v target="$*" -v jobs="$(JOBS)" -v nextest_profile="$(NEXTEST_PROFILE)" -v org="$(ORG)" "$$AWK_PARSER" Justfile | $(SHELL); \
	fi

# >>> template-only: `make new-repo` deletes this block (new-repo, template-check).
##@ Template

new-repo: ## Rename the template (NAME=<kebab> [REPO=<repo-name>] [ORG=cerebral-work])
	name="$(NAME)"
	# REPO is the GitHub repo name. It defaults to NAME, the crate prefix.
	# They differ when the repo is, for example, cas-substrate with cas-* crates.
	repo="$(or $(REPO),$(NAME))"
	org="$(ORG)"
	[[ "$$name" =~ ^[a-z][a-z0-9-]*$$ ]] || { echo "usage: make new-repo NAME=<kebab-name> [REPO=<repo-name>] [ORG=cerebral-work]" >&2; exit 2; }
	[[ "$$repo" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$$ ]] || { echo "new-repo: bad REPO" >&2; exit 2; }
	[[ "$$org" =~ ^[A-Za-z0-9][A-Za-z0-9-]*$$ ]] || { echo "new-repo: bad ORG" >&2; exit 2; }
	if [ -n "$$(git status --porcelain)" ]; then
		echo "new-repo: the working tree is not clean" >&2
		exit 1
	fi
	snake="$${name//-/_}"
	upper="$${snake^^}"
	git mv crates/skel-domain "crates/$$name-domain"
	git mv crates/skel-cli "crates/$$name-cli"
	git ls-files -z | while IFS= read -r -d '' f; do
		# vendor/ is upstream text; the new-repo skill stays verbatim as the
		# record of how this repo was made from the template.
		case "$$f" in vendor/*|Cargo.lock|.agents/skills/new-repo/*) continue ;; esac
		[ -f "$$f" ] || continue
		grep -Iq . "$$f" || continue
		# The bare token is matched on word boundaries so that a word which
		# merely contains it is never rewritten.
		sed -i \
			-e "s#cerebral-work/rust-monorepo-template#$$org/$$repo#g" \
			-e "s#rust-monorepo-template#$$repo#g" \
			-e "s/SKEL_/$${upper}_/g" \
			-e "s/skel_/$${snake}_/g" \
			-e "s/\bskel\b/$$name/g" \
			"$$f"
	done
	git rm -q -f --ignore-unmatch .github/workflows/template-check.yml
	# The template-only targets have no use in the new repo. Delete them, so
	# `make help` lists only real targets. The recipe already runs, so this is safe.
	sed -i -e '/^# >>> template-only/,/^# <<< template-only/d' -e 's/ new-repo template-check$$//' Makefile
	cargo update -w --offline 2>/dev/null || cargo update -w
	# The rename changes crate names, and rustfmt sorts imports by name, so a
	# name that sorts before `clap` (for example `cas`) leaves misordered imports.
	cargo fmt --all
	git status --short

template-check: ## Run check, spec-check, smoke, then prove the rename in a temp copy
	$(MAKE) check spec-check smoke
	tmp="$$(mktemp -d)"
	trap 'rm -rf "$$tmp"' EXIT
	git ls-files -z | tar --null -T - -cf - | tar -xf - -C "$$tmp"
	export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null
	export GIT_AUTHOR_NAME=probe GIT_AUTHOR_EMAIL=probe@example.invalid
	export GIT_COMMITTER_NAME=probe GIT_COMMITTER_EMAIL=probe@example.invalid
	export CARGO_TARGET_DIR="$(CURDIR)/target/probe"
	cd "$$tmp"
	git init -q -b development
	git add -A
	git commit -q -m probe
	# `aaa-probe` sorts before every dependency name, so the rename moves the
	# domain import to the top of the CLI's import block; fmt-check proves that
	# new-repo reformats it (measured 2026-10-08: NAME=cas failed fmt-check).
	# REPO differs from NAME, so the probe also proves that links follow the
	# repo name (measured 2026-10-08: NAME=cas wrote cerebral-work/cas, but the
	# repo was cas-substrate).
	$(MAKE) -f Makefile new-repo NAME=aaa-probe REPO=aaa-probe-repo
	cargo fmt --all --check
	grep -q 'cerebral-work/aaa-probe-repo"' Cargo.toml || { echo "template-check: Cargo.toml repository does not use REPO" >&2; exit 1; }
	if grep -qE '^(new-repo|template-check):' Makefile; then
		echo "template-check: template-only targets left in the Makefile" >&2
		exit 1
	fi
	cargo build --workspace -j $(JOBS)
	$(MAKE) -f Makefile spec-check
	if grep -rIn -E '\bskel\b|skel_|SKEL_' --exclude-dir={target,.git,vendor} . | grep -v '^\./\.agents/skills/new-repo/'; then
		echo "template-check: leftover 'skel' after rename" >&2
		exit 1
	fi
	echo "template-check: ok"
# <<< template-only
