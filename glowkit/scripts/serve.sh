#!/usr/bin/env bash
# Serve ./website on http://localhost:8787
cd "$(dirname "$0")/../website" && exec python3 -m http.server 8787
