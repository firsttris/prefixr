#!/usr/bin/env bash
# Takes the screenshots in docs/ (scripts/screenshots.mjs) from a production build of the frontend
# with the demo library from scripts/screenshots-demo.mjs: bun run screenshots
# The workflow "Screenshots" runs it in the Playwright image and commits the pictures.
set -euo pipefail
bun run build
node scripts/screenshots.mjs docs
