#!/bin/bash
# Create multiple databases in PostgreSQL
# This script is run by the postgres container on first startup

set -e

# Function to create database if it doesn't exist
create_database() {
    local database=$1
    echo "Creating database '$database' if it doesn't exist..."
    psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
        SELECT 'CREATE DATABASE $database'
        WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = '$database')\gexec
EOSQL
}

# Create logto database
create_database "logto"

echo "All databases created successfully!"

