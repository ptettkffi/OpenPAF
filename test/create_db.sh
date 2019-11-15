#!/bin/sh

# PostgreSQL
su postgres -c "psql -c \"ALTER USER postgres WITH password 'postgres'\""
psql "host=localhost port=5432 user=postgres password=postgres" -c "CREATE DATABASE openpaf"
psql "host=localhost port=5432 dbname=openpaf user=postgres password=postgres" -c "CREATE USER openpaf_user WITH PASSWORD 'openpaf123'"
psql "host=localhost port=5432 dbname=openpaf user=postgres password=postgres" -c "CREATE TABLE openpaf (id INT NOT NULL, param VARCHAR NOT NULL, numeric INT NOT NULL, nullable VARCHAR)"
psql "host=localhost port=5432 dbname=openpaf user=postgres password=postgres" -c "INSERT INTO openpaf VALUES (0, 'value', 12, NULL)"
psql "host=localhost port=5432 dbname=openpaf user=postgres password=postgres" -c "GRANT SELECT ON TABLE openpaf TO openpaf_user"
