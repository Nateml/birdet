-- Local copy of the species photo. `bird_info.image_url` is where it came from;
-- this is the filename under `app_data_dir/images/`, so the picture still shows
-- offline. Separate column (rather than overwriting image_url) keeps the source
-- URL around for re-download and for crediting the original.
ALTER TABLE bird_info ADD COLUMN image_path TEXT;
