#!/bin/bash

assets=""
for f in dist/*; do
  if [ -f "$f" ]; then
    assets="$(echo $assets) ./$f"
  fi
done

echo "uploading $assets"
gh release upload v0.0.1 -- $assets