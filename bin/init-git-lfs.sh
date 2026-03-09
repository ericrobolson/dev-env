#!/bin/bash

FOLDER_NAME="$1"

# 1. Initialize Git LFS in the repository
git lfs install

# 2. Track everything inside the specific folder (e.g., 'assets/')
# This adds a pattern to your .gitattributes file
git lfs track "$FOLDER_NAME/**"

# 3. Ensure the .gitattributes file itself is tracked by regular Git
git add .gitattributes

echo "Git LFS initialized. All future files in '$FOLDER_NAME/' will be tracked via LFS."

#Retroactive Tracking: If you already have large files committed to your history before running this script, 
# Git LFS won't automatically move them. You would need to use git lfs migrate to rewrite the history and 
# "un-bloat" the repo.

git lfs migrate import --include="$FOLDER_NAME/**" --everything

git lfs ls-files