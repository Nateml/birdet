-- Taxonomy + region descriptors on birds (for import provenance & filtering),
-- attribution on recordings (Xeno-Canto CC licences require credit), and a
-- key/value settings table for API keys and defaults.

ALTER TABLE birds ADD COLUMN ebird_code  TEXT; -- eBird species code; import dedup key
ALTER TABLE birds ADD COLUMN family      TEXT; -- family common name
ALTER TABLE birds ADD COLUMN family_sci  TEXT; -- family scientific name
ALTER TABLE birds ADD COLUMN taxon_order TEXT; -- eBird order
ALTER TABLE birds ADD COLUMN region      TEXT; -- eBird region code the bird was imported under

-- Multiple NULLs are allowed (existing seed birds), so seeds don't collide.
CREATE UNIQUE INDEX IF NOT EXISTS idx_birds_ebird_code ON birds(ebird_code);

ALTER TABLE recordings ADD COLUMN source      TEXT; -- e.g. 'xeno-canto'
ALTER TABLE recordings ADD COLUMN xc_id       TEXT; -- Xeno-Canto recording id
ALTER TABLE recordings ADD COLUMN recordist   TEXT; -- credit
ALTER TABLE recordings ADD COLUMN license_url TEXT; -- licence link

CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT
);
