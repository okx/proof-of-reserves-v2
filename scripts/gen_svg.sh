#!/bin/bash
echo "All arguments: $@"

# Access individual arguments
echo "First argument: $1"
echo "Second argument: $2"

SVG_CONTENT=$(cat <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="100" height="20">
  <rect width="100" height="20" rx="3" fill="#555"/>
  <rect x="40" width="60" height="20" rx="3" fill="$1"/>
  <text x="50%" y="50%" alignment-baseline="middle" text-anchor="middle" fill="#fff" font-size="11" font-family="Verdana">Coverage $2%</text>
</svg>
EOF
)
echo "$SVG_CONTENT" > ./target/debug/tarpaulin/coverage-badge.svg