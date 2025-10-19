-- Add migration script here
-- +migrate Up

CREATE TABLE birds (
    id SERIAL PRIMARY KEY,
    common_name VARCHAR(255) NOT NULL UNIQUE,
    scientific_name VARCHAR(255) NOT NULL UNIQUE,
    description TEXT,
    image_path VARCHAR(255)
)

CREATE TABLE recordings (
    ID SERIAL PRIMARY KEY,
    bird_ID INTEGER REFERENCES birds(id) ON DELETE CASCADE NOT NULL,
    file_path VARCHAR(255) NOT NULL,
    date_recorded TIMESTAMP,
    location VARCHAR(255)
)

CREATE TABLE packs (
    ID VARCHAR(31) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
)

CREATE TABLE pack_recordings (
    pack_ID VARCHAR(31) REFERENCES packs(ID) ON DELETE CASCADE NOT NULL,
    recording_ID INTEGER REFERENCES recordings(ID) ON DELETE CASCADE NOT NULL,
    PRIMARY KEY (pack_ID, recording_ID)
)

CREATE TABLE mastery (
    bird_id INTEGER PRIMARY KEY references birds(ID) ON DELETE CASCADE NOT NULL,
    seen INTEGER NOT NULL DEFAULT 0,
    correct INTEGER NOT NULL DEFAULT 0,
)

CREATE TABLE history (
    bird_id INTEGER REFERENCES birds(ID) ON DELETE SET NULL,
    recording_id INTEGER REFERENCES recordings(ID) ON DELETE SET NULL,
    answered_correctly BOOLEAN NOT NULL,
    answered_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
)

-- +migrate Down
DROP TABLE birds;
DROP TABLE recordings;
DROP TABLE packs;
DROP TABLE pack_recordings;
DROP TABLE mastery;
DROP TABLE history;
