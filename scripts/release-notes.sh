#!/usr/bin/env bash
# Release notes from the commit subjects since the previous release, as the
# commits are pushed directly and GitHub's own notes come from pull requests.
# Commits that only set the version are left out.
#
#   release-notes.sh <previous-tag or empty> <tag>
set -euo pipefail

previous=$1
tag=$2
repo=${GITHUB_REPOSITORY:-Visxoz/open-synapse}
version=${tag#v}

if [ -n "$previous" ]; then
    changes=$(git log --format='- %s' --reverse "$previous..$tag" \
        | grep -vE '^- (Open Synapse |Version |v)?[0-9]+\.[0-9]+\.[0-9]+$' || true)
    if [ -n "$changes" ]; then
        printf '## Changes\n\n%s\n\n' "$changes"
    fi
else
    printf 'First release of Open Synapse, a native control panel for Razer mice on Windows.\n\n'
fi

cat <<EOF
## Install

Download \`open-synapse_${version}_x64-setup.exe\` to install, or the portable exe to run from anywhere. Either one updates itself from Settings. The app is not code-signed yet, so if Windows says it protected your PC, choose **More info**, then **Run anyway**.
EOF

if [ -n "$previous" ]; then
    printf '\n**Full changelog:** https://github.com/%s/compare/%s...%s\n' "$repo" "$previous" "$tag"
fi
