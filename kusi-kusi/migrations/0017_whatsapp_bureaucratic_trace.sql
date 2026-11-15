ALTER TABLE channel_inbox DROP CONSTRAINT channel_inbox_kind_check;
ALTER TABLE channel_inbox ADD CONSTRAINT channel_inbox_kind_check
    CHECK (kind IN ('text','audio','image','status'));
