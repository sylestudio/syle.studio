-- ImageFormat is now Avif | Jpeg (no production-grade pure-Rust WebP).
ALTER TABLE photo_variants
    DROP CONSTRAINT photo_variants_format_check;
ALTER TABLE photo_variants
    ADD CONSTRAINT photo_variants_format_check
    CHECK (format IN ('avif', 'jpeg'));
