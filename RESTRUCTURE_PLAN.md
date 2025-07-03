# Repository Restructure Plan

## Completed
✅ **Phase 1: Repository Cleanup**
- Removed committed worktree directories: `deployment-worktree/`, `fix-podman-ci-worktree/`, `podman-fix-worktree/`
- Added `*worktree/` to `.gitignore`
- Committed and pushed changes to `dominic/delete-committed-worktree-dirs` branch

## Manual Steps Required

### Phase 2: Directory Restructure
The following steps need to be executed manually from `/home/dominic/code/`:

```bash
# 1. Create new parent directory structure
mkdir -p /home/dominic/code/velib-mcp-new

# 2. Move existing repository to subdirectory
mv /home/dominic/code/velib-mcp /home/dominic/code/velib-mcp-new/velib-mcp

# 3. Update the parent directory name
mv /home/dominic/code/velib-mcp-new /home/dominic/code/velib-mcp

# 4. Clean up and recreate worktrees
cd /home/dominic/code/velib-mcp/velib-mcp
git worktree prune
```

### Phase 3: Verify New Structure
After restructure, the directory layout should be:
```
~/code/velib-mcp/
├── velib-mcp/              # Main repository (relocated)
│   ├── CLAUDE.md           # Configuration Claude partagée
│   ├── src/                # Code source
│   ├── docs/               # Documentation
│   └── ...                 # Project files
├── branch1/                # Future worktrees
├── branch2/                # Future worktrees
└── ...                     # Other parallel worktrees
```

### Phase 4: Test Worktree Creation
```bash
cd /home/dominic/code/velib-mcp/velib-mcp

# Create test worktree in adjacent directory
git worktree add ../test-branch main
cd ../test-branch
ln -s ../velib-mcp/CLAUDE.md CLAUDE.md

# Verify it works
ls -la
git status

# Clean up test
cd ../velib-mcp
git worktree remove ../test-branch
```

## Benefits of New Structure
1. **Clean GitHub UI**: No more worktree directories in repository
2. **Organized Development**: Adjacent worktrees for parallel work
3. **Consistent Paths**: All worktrees at same directory level
4. **Improved Workflow**: Easier navigation between branches

## Next Steps
1. Execute manual restructure steps above
2. Update CLAUDE.md to reflect new paths
3. Update any documentation referencing old paths
4. Create PR to merge worktree cleanup changes