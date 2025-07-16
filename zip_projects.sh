#!/bin/bash

# Script to zip each Rust project in the workspace
# Usage: ./zip_projects.sh

set -e  # Exit on any error

# Define the base directory (script location)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "Starting to zip functions..."

# Find all directories containing Cargo.toml, excluding the local folder
while IFS= read -r -d '' cargo_file; do
    project_dir="$(dirname "$cargo_file")"
    project_name="$(basename "$project_dir")"

    # Skip the local folder
    if [[ "$project_name" == "local" ]]; then
        continue
    fi

    # Skip if it's the root Cargo.toml (if any)
    if [[ "$project_dir" == "$SCRIPT_DIR" ]]; then
        continue
    fi

    echo "Processing $project_name..."

    # Change to project directory
    cd "$project_dir"

    # Create zip file with project name
    zip_file="../${project_name}.zip"

    # Remove existing zip if it exists
    if [[ -f "$zip_file" ]]; then
        echo "  Removing existing $zip_file"
        rm "$zip_file"
    fi

    # Create the zip file
    echo "  Creating $zip_file"
    zip -r "$zip_file" Cargo.lock Cargo.toml src/*

    echo "  ✓ Created $zip_file"

done < <(find "$SCRIPT_DIR" -maxdepth 2 -name "Cargo.toml" -print0)

echo "All projects processed!"
