-- Per-pack icon (an emoji). NULL falls back to a rotating default in the UI.
ALTER TABLE packs ADD COLUMN icon TEXT;
