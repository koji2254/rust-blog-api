-- Existing test posts have no owner, so clear them first
DELETE FROM posts;

ALTER TABLE posts
    ADD COLUMN author_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE;

CREATE INDEX posts_author_id_idx ON posts(author_id);
