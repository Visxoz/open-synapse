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
    printf 'First release of Open Synapse, a native control panel for Razer mice on Windows, Linux and macOS.\n\n'
fi

cat <<EOF
## Install

**Windows:** download \`open-synapse_${version}_x64-setup.exe\` to install, or the portable exe to run from anywhere. The app is not code-signed yet, so if Windows says it protected your PC, choose **More info**, then **Run anyway**.

**Linux:** unpack \`open-synapse_${version}_linux-x86_64.tar.gz\` and run \`./install.sh\`, which installs the app for you and, with sudo, the udev rule that lets it open the mouse.

**macOS:** unzip \`open-synapse_${version}_macos-aarch64.zip\` (Apple silicon) or \`_macos-x86_64.zip\` (Intel) and move **Open Synapse** to Applications. It is not notarised, so the first time, open it from **System Settings ▸ Privacy & Security ▸ Open Anyway**.

Linux and macOS support is new and has not yet been tried with a mouse attached; please [open an issue](https://github.com/${repo}/issues) to say how it went.

Every copy updates itself from Settings.
EOF

if [ -n "$previous" ]; then
    printf '\n**Full changelog:** https://github.com/%s/compare/%s...%s\n' "$repo" "$previous" "$tag"
fi
