-- Spaced-repetition (SM-2 + learning steps) scheduling state, per bird.
-- The `mastery` table already keys on bird_ID, so it doubles as the card table.
ALTER TABLE mastery ADD COLUMN ease REAL NOT NULL DEFAULT 2.5;          -- SM-2 ease factor
ALTER TABLE mastery ADD COLUMN interval_days REAL NOT NULL DEFAULT 0;   -- current review interval
ALTER TABLE mastery ADD COLUMN due_at TIMESTAMP;                        -- next review time; NULL = new
ALTER TABLE mastery ADD COLUMN reps INTEGER NOT NULL DEFAULT 0;         -- successful reviews in a row-ish
ALTER TABLE mastery ADD COLUMN lapses INTEGER NOT NULL DEFAULT 0;       -- times a mature card was failed
ALTER TABLE mastery ADD COLUMN learning_step INTEGER NOT NULL DEFAULT 0;-- index into the learning steps
ALTER TABLE mastery ADD COLUMN state TEXT NOT NULL DEFAULT 'new';       -- new | learning | review
ALTER TABLE mastery ADD COLUMN last_reviewed TIMESTAMP;                 -- clock of the last answer

-- Existing progress predates scheduling: make already-seen birds due immediately
-- as review cards so they re-enter the rotation rather than counting as brand new.
UPDATE mastery SET state = 'review', due_at = datetime('now') WHERE seen > 0;
