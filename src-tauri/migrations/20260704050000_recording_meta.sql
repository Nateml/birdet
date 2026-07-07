-- Xeno-Canto per-recording metadata surfaced in the recording manager.
ALTER TABLE recordings ADD COLUMN quality  TEXT; -- XC quality rating A–E
ALTER TABLE recordings ADD COLUMN rec_type TEXT; -- vocalization type, e.g. song/call
