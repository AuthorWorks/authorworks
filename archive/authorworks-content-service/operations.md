# AuthorWorks Operations Guide

This guide explains how to work with the AuthorWorks repository structure and how to keep your work in sync across repositories.

## Repository Structure

AuthorWorks uses a multi-repository architecture:

- **Main Repository**: `~/git/authorworks/` - Contains all specifications and synchronization scripts
- **Service Repositories**: Located in `~/git/aw/authorworks-[service-name]/` - Each component has its own dedicated repository

## Setting Up Your Environment

### Prerequisites

- Git
- GitHub CLI
- Bash shell
- GitHub Personal Access Token with appropriate permissions

### Initial Setup

1. Clone the main repository:
   ```bash
   git clone https://github.com/authorworks/authorworks.git ~/git/authorworks
   ```

2. Set up a GitHub security token:
   
   **Option 1: Using environment variables (recommended)**
   - Add the following to your `.zshrc` or `.bashrc` file:
     ```bash
     export GITHUB_USERNAME='your-username'
     export GITHUB_ACCESS_TOKEN='your-github-token'
     ```
   - Generate the token from GitHub with the following permissions:
     - `repo` (Full control of private repositories)
     - `workflow` (Update GitHub Action workflows)

3. Run the repository setup script:
   ```bash
   cd ~/git/authorworks
   chmod +x scripts/repo_setup.sh
   ./scripts/repo_setup.sh all
   ```

This will:
- Create all repositories in the GitHub organization (if they don't exist)
- Set the SYNC_TOKEN secret in each repository
- Clone the repositories locally to `~/git/aw/`
- Copy specifications to the appropriate repositories
- Set up GitHub Actions workflows for bi-directional synchronization

## Working with Repositories

### 1. Making Changes to Specifications

#### A. When to Edit in the Main Repository

Edit specifications in the main repository when:
- Making changes that affect multiple services
- Defining new cross-cutting concerns
- Establishing platform-wide standards

**Workflow**:
1. Navigate to the main repository: `cd ~/git/authorworks`
2. Make changes to the appropriate files in `specs/` or `docs/`
3. Commit and push your changes: `git commit -m "Your message" && git push`
4. The GitHub Actions workflow will automatically sync changes to all service repositories

#### B. When to Edit in Service Repositories

Edit specifications in a service repository when:
- Making service-specific changes
- Implementing service-specific features
- Documenting service-specific behavior

**Workflow**:
1. Navigate to the service repository: `cd ~/git/aw/authorworks-[service-name]`
2. Make changes to files in `specs/` or `docs/`
3. Commit and push your changes: `git commit -m "Your message" && git push`
4. The GitHub Actions workflow will automatically sync changes back to the main repository

### 2. Keeping Repositories in Sync

The bi-directional synchronization is handled automatically by GitHub Actions workflows, but you can also manually trigger a sync:

```bash
cd ~/git/authorworks
./scripts/repo_setup.sh sync
```

### 3. Adding New Specifications

When adding new specifications:

1. Determine the appropriate location:
   - `/specs/1-infrastructure/` - For infrastructure and cross-cutting concerns
   - `/specs/2-services/` - For service-specific specifications
   - `/specs/3-business-logic/` - For business logic and process flows
   - `/specs/4-ui/` - For UI-related specifications

2. Use consistent naming conventions:
   - Infrastructure: `1X-[name].md` (e.g., `1A-repository-structure.md`)
   - Services: `2X-[service-name]-service.md` (e.g., `2B-user-service.md`)
   - Business Logic: `3X-[name].md` (e.g., `3A-repository-distribution.md`)
   - UI: `4X-[name].md` (e.g., `4A-component-library.md`)

3. Reference other specifications using relative links:
   ```markdown
   See [Repository Structure](1-infrastructure/1A-repository-structure.md) for details.
   ```

## Troubleshooting

### Handling Sync Conflicts

If conflicts occur during synchronization:

1. Check the GitHub Actions logs to identify the conflict
2. Manually resolve the conflict in the affected repository
3. Push the resolution
4. Re-run the synchronization script if needed

### Common Issues

1. **Authentication Issues**:
   - Ensure your GitHub token has the correct permissions
   - Verify the `SYNC_TOKEN` secret is set in all repositories
   - Check that your token hasn't expired

2. **Workflow Failures**:
   - Check the GitHub Actions logs for error details
   - Verify that paths in the workflow files are correct
   - Ensure that the GitHub Actions runner has access to all repositories

3. **Local Repository Issues**:
   - Make sure all repositories are cloned to the expected locations
   - Pull the latest changes before running synchronization operations
   - Resolve any local conflicts before pushing changes

## Best Practices

1. **Update Specifications First**: When making significant changes, update the specifications first and get them reviewed before implementing the changes.

2. **Reference Specifications in Code**: When implementing features, include references to the relevant specifications in your code comments.

3. **Keep Synced Copies Read-Only**: Consider the copies of specifications in service repositories as read-only references for implementation guidance.

4. **Organize Changes**: Group related changes into cohesive commits with clear commit messages.

5. **Regular Updates**: Regularly pull changes from all repositories to stay up-to-date with the latest specifications.

6. **Documentation**: Keep the documentation up-to-date as specifications evolve.

## Script Reference

The `repo_setup.sh` script accepts the following commands:

- `create-repos`: Create all repositories in the GitHub organization
- `clone`: Clone all repositories locally
- `sync`: Synchronize specifications and documentation
- `create-workflows`: Create GitHub workflow files
- `create-readme`: Create README.md files
- `update-readme`: Update README files with specification references
- `update-dev-sections`: Update development sections in README files
- `distribute-src`: Distribute source code to repositories
- `all`: Run all commands in sequence

Example:
```bash
./scripts/repo_setup.sh sync
```

## Additional Resources

- [GitHub Flow Documentation](https://docs.github.com/en/get-started/quickstart/github-flow)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Specifications Overview](specs/0-overview.md)
- [Specifications Sync Strategy](docs/specs-sync-strategy.md) 