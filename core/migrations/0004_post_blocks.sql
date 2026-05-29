-- Posts move from a Markdown string to a structured block document
-- (Vec<Block> in syle-types), stored as JSON text. The renderer (syle-render)
-- turns it into HTML for both the public site and the CRM preview.

ALTER TABLE blog_posts ADD COLUMN body_blocks TEXT NOT NULL DEFAULT '[]';

-- Best-effort preserve any existing post text as a single paragraph block so
-- nothing is silently dropped (formatting is flattened; words are kept).
UPDATE blog_posts
SET body_blocks = json_build_array(
    json_build_object(
        'type', 'paragraph',
        'id', 'b-' || replace(id::text, '-', ''),
        'content', json_build_array(json_build_object('text', body_md))
    )
)::text
WHERE body_blocks = '[]' AND body_md <> '';

-- body_md is now legacy: new writes target body_blocks and ignore it.
ALTER TABLE blog_posts ALTER COLUMN body_md DROP NOT NULL;
ALTER TABLE blog_posts ALTER COLUMN body_md SET DEFAULT '';
