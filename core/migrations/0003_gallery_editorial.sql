-- Editorial narrative fields the public project page renders (lede, closing
-- notes, discipline, year). Existing rows default to empty / NULL year.
ALTER TABLE galleries ADD COLUMN description TEXT NOT NULL DEFAULT '';
ALTER TABLE galleries ADD COLUMN notes       TEXT NOT NULL DEFAULT '';
ALTER TABLE galleries ADD COLUMN category    TEXT NOT NULL DEFAULT '';
ALTER TABLE galleries ADD COLUMN year        INTEGER;
