#!/bin/bash

# AuthorWorks Repository Synchronization Script
# This script manages all git repositories in the AuthorWorks workspace
# It handles pulling, committing, pushing, and merge conflict resolution

set -e  # Exit on any error (can be overridden for specific operations)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
WORKSPACE_ROOT="/Users/leo/git/AuthorWorks"
LOG_FILE="${WORKSPACE_ROOT}/sync_repos.log"
TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')

# Initialize log file
echo "=== Repository Sync Started at $TIMESTAMP ===" >> "$LOG_FILE"

# Function to print colored output
print_status() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
    echo "[$TIMESTAMP] $message" >> "$LOG_FILE"
}

# Function to handle errors gracefully
handle_error() {
    local repo_path=$1
    local operation=$2
    local error_msg=$3
    
    print_status "$RED" "❌ Error in $repo_path during $operation: $error_msg"
    echo "CONTINUE? (y/n/s=skip this repo): "
    read -r response
    case $response in
        [Yy]* ) return 0;;
        [Ss]* ) return 1;;
        * ) exit 1;;
    esac
}

# Function to check if directory is a git repository
is_git_repo() {
    local dir=$1
    [ -d "$dir/.git" ]
}

# Function to get repository status
get_repo_status() {
    local repo_path=$1
    cd "$repo_path"
    
    local branch=$(git branch --show-current)
    local status=$(git status --porcelain)
    local behind=$(git rev-list --count HEAD..@{u} 2>/dev/null || echo "0")
    local ahead=$(git rev-list --count @{u}..HEAD 2>/dev/null || echo "0")
    
    echo "$branch|$status|$behind|$ahead"
}

# Function to stash changes if needed
stash_changes() {
    local repo_path=$1
    cd "$repo_path"
    
    if [[ -n $(git status --porcelain) ]]; then
        print_status "$YELLOW" "  📦 Stashing uncommitted changes..."
        git stash push -m "Auto-stash before sync $(date '+%Y-%m-%d %H:%M:%S')" || return 1
        echo "stashed"
    else
        echo "clean"
    fi
}

# Function to pop stashed changes
pop_stash() {
    local repo_path=$1
    cd "$repo_path"
    
    if git stash list | grep -q "Auto-stash before sync"; then
        print_status "$YELLOW" "  📤 Restoring stashed changes..."
        git stash pop || {
            print_status "$RED" "  ⚠️  Stash pop failed - you may need to resolve conflicts manually"
            return 1
        }
    fi
}

# Function to commit staged and unstaged changes
commit_changes() {
    local repo_path=$1
    cd "$repo_path"
    
    # Add all changes
    git add . || return 1
    
    # Check if there are changes to commit
    if git diff --cached --quiet; then
        print_status "$BLUE" "  ✅ No changes to commit"
        return 0
    fi
    
    # Create commit message
    local commit_msg="Auto-sync: Updates from $(hostname) at $(date '+%Y-%m-%d %H:%M:%S')"
    
    print_status "$GREEN" "  💾 Committing changes..."
    git commit -m "$commit_msg" || return 1
}

# Function to handle merge conflicts
resolve_merge_conflicts() {
    local repo_path=$1
    cd "$repo_path"
    
    print_status "$YELLOW" "  ⚠️  Merge conflicts detected!"
    print_status "$CYAN" "  📋 Conflicted files:"
    git diff --name-only --diff-filter=U | while read -r file; do
        print_status "$CYAN" "    - $file"
    done
    
    echo ""
    echo "Choose conflict resolution strategy:"
    echo "1) Open merge tool (if configured)"
    echo "2) Accept all incoming changes (theirs)"
    echo "3) Accept all local changes (ours)"
    echo "4) Skip this repository"
    echo "5) Manual resolution (exit to terminal)"
    
    read -p "Enter choice (1-5): " choice
    
    case $choice in
        1)
            if git config merge.tool >/dev/null 2>&1; then
                git mergetool
                git commit --no-edit
            else
                print_status "$RED" "No merge tool configured. Please set one with 'git config merge.tool <tool>'"
                return 1
            fi
            ;;
        2)
            git checkout --theirs .
            git add .
            git commit --no-edit
            ;;
        3)
            git checkout --ours .
            git add .
            git commit --no-edit
            ;;
        4)
            git merge --abort
            return 1
            ;;
        5)
            print_status "$CYAN" "Dropping to shell in $repo_path"
            print_status "$CYAN" "Run 'exit' when done resolving conflicts"
            bash
            ;;
        *)
            print_status "$RED" "Invalid choice"
            return 1
            ;;
    esac
}

# Function to sync a single repository
sync_repository() {
    local repo_path=$1
    local repo_name=$(basename "$repo_path")
    
    print_status "$PURPLE" "\n🔄 Processing: $repo_name"
    print_status "$BLUE" "  📍 Path: $repo_path"
    
    # Convert relative path to absolute path
    if [[ "$repo_path" == ./* ]]; then
        repo_path="$WORKSPACE_ROOT/${repo_path#./}"
    fi
    
    # Change to repository directory
    if ! cd "$repo_path"; then
        handle_error "$repo_path" "cd" "Cannot access directory"
        return $?
    fi
    
    # Verify it's a git repository
    if ! is_git_repo "$repo_path"; then
        print_status "$YELLOW" "  ⚠️  Not a git repository, skipping..."
        return 0
    fi
    
    # Get current status
    local status_info=$(get_repo_status "$repo_path")
    IFS='|' read -r branch status_output behind ahead <<< "$status_info"
    
    print_status "$BLUE" "  🌿 Branch: $branch"
    
    # Check for uncommitted changes
    if [[ -n "$status_output" ]]; then
        print_status "$YELLOW" "  📝 Uncommitted changes detected:"
        echo "$status_output" | while read -r line; do
            [[ -n "$line" ]] && print_status "$YELLOW" "    $line"
        done
        
        echo "What would you like to do with uncommitted changes?"
        echo "1) Commit them automatically"
        echo "2) Stash them temporarily"
        echo "3) Skip this repository"
        read -p "Enter choice (1-3): " choice
        
        case $choice in
            1)
                if ! commit_changes "$repo_path"; then
                    handle_error "$repo_path" "commit" "Failed to commit changes"
                    [[ $? -eq 1 ]] && return 0
                fi
                ;;
            2)
                local stash_result=$(stash_changes "$repo_path")
                if [[ "$stash_result" != "stashed" && "$stash_result" != "clean" ]]; then
                    handle_error "$repo_path" "stash" "Failed to stash changes"
                    [[ $? -eq 1 ]] && return 0
                fi
                ;;
            3)
                print_status "$YELLOW" "  ⏭️  Skipping repository"
                return 0
                ;;
            *)
                print_status "$RED" "Invalid choice, skipping repository"
                return 0
                ;;
        esac
    fi
    
    # Fetch latest changes
    print_status "$BLUE" "  📥 Fetching latest changes..."
    if ! git fetch origin; then
        handle_error "$repo_path" "fetch" "Failed to fetch from origin"
        [[ $? -eq 1 ]] && return 0
    fi
    
    # Check if we're behind
    behind=$(git rev-list --count HEAD..@{u} 2>/dev/null || echo "0")
    if [[ "$behind" -gt 0 ]]; then
        print_status "$YELLOW" "  ⬇️  $behind commits behind origin"
        print_status "$BLUE" "  🔄 Pulling changes..."
        
        if ! git pull origin "$branch"; then
            if git status | grep -q "You have unmerged paths"; then
                if ! resolve_merge_conflicts "$repo_path"; then
                    print_status "$YELLOW" "  ⏭️  Skipping due to unresolved conflicts"
                    return 0
                fi
            else
                handle_error "$repo_path" "pull" "Failed to pull changes"
                [[ $? -eq 1 ]] && return 0
            fi
        fi
    else
        print_status "$GREEN" "  ✅ Already up to date with origin"
    fi
    
    # Restore stashed changes if any
    if [[ "${stash_result:-}" == "stashed" ]]; then
        pop_stash "$repo_path"
    fi
    
    # Check if we're ahead and need to push
    ahead=$(git rev-list --count @{u}..HEAD 2>/dev/null || echo "0")
    if [[ "$ahead" -gt 0 ]]; then
        print_status "$YELLOW" "  ⬆️  $ahead commits ahead of origin"
        print_status "$BLUE" "  📤 Pushing changes..."
        
        if ! git push origin "$branch"; then
            handle_error "$repo_path" "push" "Failed to push changes"
            [[ $? -eq 1 ]] && return 0
        fi
        print_status "$GREEN" "  ✅ Successfully pushed $ahead commits"
    else
        print_status "$GREEN" "  ✅ Nothing to push"
    fi
    
    print_status "$GREEN" "  🎉 Repository sync completed successfully"
}

# Main function
main() {
    print_status "$CYAN" "🚀 Starting AuthorWorks Repository Synchronization"
    print_status "$BLUE" "📁 Workspace: $WORKSPACE_ROOT"
    print_status "$BLUE" "📋 Log file: $LOG_FILE"
    
    # Change to workspace root
    cd "$WORKSPACE_ROOT"
    
    # Find all git repositories
    print_status "$BLUE" "🔍 Discovering git repositories..."
    local repos=()
    while IFS= read -r -d '' repo; do
        repo_dir=$(dirname "$repo")
        repos+=("$repo_dir")
    done < <(find . -name ".git" -type d -print0)
    
    print_status "$GREEN" "📊 Found ${#repos[@]} git repositories"
    
    # List repositories for confirmation
    print_status "$CYAN" "📋 Repositories to sync:"
    for repo in "${repos[@]}"; do
        print_status "$CYAN" "  - $(basename "$repo")"
    done
    
    echo ""
    read -p "Continue with synchronization? (y/n): " confirm
    if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
        print_status "$YELLOW" "❌ Synchronization cancelled by user"
        exit 0
    fi
    
    # Sync each repository
    local success_count=0
    local total_count=${#repos[@]}
    
    for repo in "${repos[@]}"; do
        if sync_repository "$repo"; then
            ((success_count++))
        fi
    done
    
    # Summary
    print_status "$CYAN" "\n📊 Synchronization Summary:"
    print_status "$GREEN" "✅ Successfully synced: $success_count/$total_count repositories"
    
    if [[ $success_count -lt $total_count ]]; then
        local failed_count=$((total_count - success_count))
        print_status "$YELLOW" "⚠️  Issues encountered: $failed_count repositories"
        print_status "$BLUE" "📋 Check $LOG_FILE for detailed information"
    fi
    
    print_status "$CYAN" "🎉 Repository synchronization completed!"
}

# Function to show repository status without making changes
show_repo_status() {
    local repo_path=$1
    local repo_name=$(basename "$repo_path")
    
    # Convert relative path to absolute path
    if [[ "$repo_path" == ./* ]]; then
        repo_path="$WORKSPACE_ROOT/${repo_path#./}"
    fi
    
    if ! cd "$repo_path" 2>/dev/null; then
        print_status "$RED" "\n📁 $repo_name"
        print_status "$RED" "  ❌ Cannot access directory: $repo_path"
        return 1
    fi
    
    # Verify it's a git repository
    if ! is_git_repo "$repo_path"; then
        print_status "$YELLOW" "\n📁 $repo_name"
        print_status "$YELLOW" "  ⚠️  Not a git repository, skipping..."
        return 0
    fi
    
    # Get basic info
    local branch=$(git branch --show-current 2>/dev/null || echo "unknown")
    local status=$(git status --porcelain 2>/dev/null)
    local behind=$(git rev-list --count HEAD..@{u} 2>/dev/null || echo "?")
    local ahead=$(git rev-list --count @{u}..HEAD 2>/dev/null || echo "?")
    local last_commit=$(git log -1 --pretty=format:"%h %s" 2>/dev/null || echo "No commits")
    
    print_status "$PURPLE" "\n📁 $repo_name"
    print_status "$BLUE" "  🌿 Branch: $branch"
    print_status "$BLUE" "  📝 Last commit: $last_commit"
    
    # Check status
    if [[ -n "$status" ]]; then
        print_status "$YELLOW" "  ⚠️  Uncommitted changes:"
        echo "$status" | while read -r line; do
            [[ -n "$line" ]] && print_status "$YELLOW" "    $line"
        done
    else
        print_status "$GREEN" "  ✅ Working directory clean"
    fi
    
    # Check sync status
    if [[ "$behind" != "?" && "$ahead" != "?" ]]; then
        if [[ "$behind" -gt 0 ]]; then
            print_status "$RED" "  ⬇️  Behind origin by $behind commits"
        fi
        if [[ "$ahead" -gt 0 ]]; then
            print_status "$YELLOW" "  ⬆️  Ahead of origin by $ahead commits"
        fi
        if [[ "$behind" -eq 0 && "$ahead" -eq 0 ]]; then
            print_status "$GREEN" "  ✅ In sync with origin"
        fi
    else
        print_status "$CYAN" "  🔍 Unable to check sync status (no remote or network issue)"
    fi
}

# Function to show status of all repositories
show_all_status() {
    print_status "$CYAN" "🔍 AuthorWorks Repository Status Check"
    print_status "$BLUE" "📁 Workspace: $WORKSPACE_ROOT"
    
    cd "$WORKSPACE_ROOT"
    
    # Find all git repositories
    local repos=()
    while IFS= read -r -d '' repo; do
        repo_dir=$(dirname "$repo")
        repos+=("$repo_dir")
    done < <(find . -name ".git" -type d -print0)
    
    print_status "$GREEN" "📊 Checking ${#repos[@]} repositories..."
    
    for repo in "${repos[@]}"; do
        show_repo_status "$repo"
    done
    
    print_status "$CYAN" "\n🎉 Status check completed!"
}

# Handle script arguments
case "${1:-}" in
    --help|-h)
        echo "AuthorWorks Repository Synchronization Script"
        echo ""
        echo "Usage: $0 [OPTIONS]"
        echo ""
        echo "Options:"
        echo "  --help, -h     Show this help message"
        echo "  --status       Show status of all repositories without making changes"
        echo "  --list         List all git repositories"
        echo ""
        echo "This script will:"
        echo "  1. Find all git repositories in the workspace"
        echo "  2. Handle uncommitted changes (commit or stash)"
        echo "  3. Pull latest changes from origin"
        echo "  4. Resolve merge conflicts if necessary"
        echo "  5. Push any local commits to origin"
        echo ""
        exit 0
        ;;
    --status)
        show_all_status
        exit 0
        ;;
    --list)
        print_status "$BLUE" "📋 Git repositories in workspace:"
        find "$WORKSPACE_ROOT" -name ".git" -type d | while read -r git_dir; do
            repo_dir=$(dirname "$git_dir")
            repo_name=$(basename "$repo_dir")
            print_status "$CYAN" "  - $repo_name ($repo_dir)"
        done
        exit 0
        ;;
esac

# Run main function
main "$@"
