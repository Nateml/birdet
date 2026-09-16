-- Attribution links for the species photo. `image_credit` and `image_license`
-- are only names ("Alan Manson", "CC BY-SA 2.0"); a CC licence also wants a
-- link to the licence itself and, where the source supplies one, a link back to
-- the photo. Kept as columns rather than derived in the UI because the licence
-- URL is what the source (Commons `extmetadata`, iNaturalist's licence code)
-- actually asserted at fetch time.
ALTER TABLE bird_info ADD COLUMN image_license_url TEXT;
ALTER TABLE bird_info ADD COLUMN image_source_url TEXT;
