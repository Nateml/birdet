-- Educational blurbs for the birds in the library: what the species looks like,
-- where it lives, how it behaves, and — most useful for an ear-trainer — how its
-- voice is usually described. Kept in its own table rather than as columns on
-- `birds` because it is refetchable, provenance-carrying and mostly NULL: the
-- birds row is the identity, this is a cache of what we could find about it.
--
-- Source text is Wikipedia (CC BY-SA 4.0), so `source_url` + `license` must be
-- shown wherever the text is. `user_notes` is the user's own field notes and is
-- never touched by a refetch.
CREATE TABLE IF NOT EXISTS bird_info (
    bird_id       INTEGER PRIMARY KEY REFERENCES birds(ID) ON DELETE CASCADE,
    summary       TEXT,    -- lead paragraph(s)
    appearance    TEXT,    -- "Description"
    habitat       TEXT,    -- "Distribution and habitat"
    behaviour     TEXT,    -- "Behaviour" / "Ecology" / "Diet" / "Breeding"
    voice         TEXT,    -- "Vocalisation" / "Voice" / "Calls" — the money field
    conservation  TEXT,    -- IUCN status name, e.g. "Least Concern"
    image_url     TEXT,    -- remote photo (not downloaded yet)
    image_credit  TEXT,
    image_license TEXT,
    source        TEXT,    -- 'wikipedia' | 'user'
    source_url    TEXT,
    license       TEXT,    -- e.g. 'CC BY-SA 4.0'
    fetched_at    TIMESTAMP,
    user_notes    TEXT
);
