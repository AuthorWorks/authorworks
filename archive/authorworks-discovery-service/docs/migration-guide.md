# AuthorWorks Repository Migration Guide

This guide will help you migrate from the current single repository structure to the new multi-repository structure for AuthorWorks.

## Migration Overview

We are transitioning from a monolithic repository (`~/git/authorworks/`) to a multi-repository structure with `~/git/aw/` as the base directory. Each component of the AuthorWorks platform will have its own dedicated repository.

## Migration Steps

### 1. Backup Current Repository

Before proceeding, create a backup of your current repository:

```bash
cp -r ~/git/authorworks ~/git/authorworks-backup
```

### 2. Create the New Base Directory

Create the new base directory for all AuthorWorks repositories:

```bash
mkdir -p ~/git/aw
```

### 3. Set Up GitHub Personal Access Token (PAT)

**Option 1: Using environment variables (recommended)**

1. Add the following to your `.zshrc` or `.bashrc` file:
   ```bash
   export GITHUB_USERNAME='your-username'
   export GITHUB_ACCESS_TOKEN='your-github-token'
   ```
   - Generate the token from GitHub: https://github.com/settings/tokens
   - Make sure to select the "repo" scope

2. Reload your shell configuration:
   ```bash
   source ~/.zshrc  # or ~/.bashrc
   ```

3. The setup script will automatically use these credentials and configure all repositories.

**Option 2: Manual setup**

1. Go to GitHub: https://github.com/settings/tokens
2. Click "Generate new token" → "Generate new token (classic)"
3. Name: "AuthorWorks Repository Sync"
4. Expiration: Set as needed (90 days recommended)
5. Select repositories: All repositories (or specific repositories within the AuthorWorks organization)
6. Permissions: Full control of private repositories
7. Click "Generate token"
8. Copy the token (this is your only chance to view it)
9. Proceed to step 4 below to add it as a secret to each repository

### 4. Configure GitHub Repository Secrets

If using Option 2 (manual setup), add the token as a secret to each repository:

1. Go to each repository's settings page:
   - Main repository: https://github.com/authorworks/authorworks/settings/secrets/actions
   - Each service repository: https://github.com/authorworks/authorworks-[service]/settings/secrets/actions
2. Click "New repository secret"
3. Name: `SYNC_TOKEN`
4. Value: paste the token you generated
5. Click "Add secret"

Repeat for all repositories, or use GitHub's organization secrets feature to set it once for all repositories.

If using Option 1 (environment variables), you can also set the secrets automatically using:

```bash
cd ~/git/authorworks
chmod +x scripts/repo_setup.sh
./scripts/repo_setup.sh secrets
```

### 5. Run the Repository Setup Script

Run the repository setup script to create and populate all repositories:

```bash
cd ~/git/authorworks
chmod +x scripts/repo_setup.sh
./scripts/repo_setup.sh sync
```

This will:
- Create all repositories in the GitHub organization (if they don't exist)
- Clone them locally to `~/git/aw/`
- Copy specifications to the appropriate repositories
- Create README files
- Commit and push changes

### 6. Verify Migration

Verify that all repositories have been created and populated correctly:

```bash
ls -la ~/git/aw
```

You should see all the AuthorWorks repositories listed.

### 7. Update Your Development Workflow

Update your development workflow to work with the new repository structure:

- Navigate to the specific repository you want to work on, e.g., `cd ~/git/aw/authorworks-content-service`
- Make changes, commit, and push as usual
- For changes that affect multiple repositories, make sure to update each repository individually

### 8. Repository Overview

Here's an overview of all repositories in the new structure:

| Repository | Description |
|------------|-------------|
| authorworks | Platform-wide infrastructure and configurations |
| authorworks-content-service | Content management and storage |
| authorworks-user-service | User authentication and management |
| authorworks-subscription-service | Subscription and payment handling |
| authorworks-storage-service | File storage and retrieval |
| authorworks-editor-service | Text and media editing capabilities |
| authorworks-messaging-service | Internal and user notifications |
| authorworks-discovery-service | Content discovery and search |
| authorworks-graphics-service | Image and graphics processing |
| authorworks-audio-service | Audio processing and management |
| authorworks-video-service | Video processing and management |
| authorworks-ui-shell | Main UI framework and shell |
| authorworks-ui | UI components and libraries |
| authorworks-docs | Documentation and specifications |

## Working with GitHub Actions

The new structure uses GitHub Actions for automating specification synchronization. Whenever changes are pushed to the specifications in the main repository, they will be automatically synchronized to the relevant repositories.

## Troubleshooting

### Repository Creation Issues

If you encounter issues with repository creation, you can create them manually through the GitHub web interface and then run the script again with:

```bash
./scripts/repo_setup.sh clone
./scripts/repo_setup.sh sync
```

### Permissions Issues

If you encounter permissions issues, make sure you have:
1. Installed and authenticated with the GitHub CLI
2. Have sufficient permissions to create and push to repositories in the AuthorWorks organization

### Repository Already Exists Locally

If a repository already exists locally in the `~/git/aw/` directory, the script will pull the latest changes rather than cloning again.

## Future Maintenance

For future maintenance of specifications, continue to make changes in the main specification repository and they will be automatically synchronized to the individual repositories through GitHub Actions. 