-- When 1, the spawn path skips automatic git worktree creation for this
-- session. The agent runs directly in the project root (or existing
-- worktree_path if one was set before this flag was introduced).
-- Default 0 preserves the existing auto-worktree behaviour for all
-- pre-existing and newly-created sessions.
ALTER TABLE agent_sessions ADD COLUMN no_worktree INTEGER NOT NULL DEFAULT 0;
