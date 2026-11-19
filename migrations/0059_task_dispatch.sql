-- 0059 multi-desk task dispatch: assignee + desk on tasks. Forward-only.
ALTER TABLE tasks
  ADD COLUMN assignee VARCHAR(120) NOT NULL DEFAULT '',
  ADD COLUMN desk VARCHAR(60) NOT NULL DEFAULT '';
