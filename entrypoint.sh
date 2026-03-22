#!/bin/bash
set -e

# Ensure data directory exists
if [ ! -d "data" ]; then
    echo "Creating data directory..."
    mkdir -p data
fi

# Initialize database if it doesn't exist
if [ ! -f "data/db.sqlite3" ]; then
    echo "Initializing database from db.sql..."
    sqlite3 data/db.sqlite3 < schema.sql
    echo "Database initialized."
else
    echo "Database exists, skipping initialization."
fi

# Execute the application
echo "Starting..."
exec ./nodecask
