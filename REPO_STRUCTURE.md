AuthorWorks Umbrella Repository

Overview
- This directory serves as the umbrella repo containing all AuthorWorks sub-repositories as git submodules.
- Use scripts/plan-submodules.sh to see mapping of local folders to their remotes.
- Use scripts/convert-to-submodules.sh --apply to convert nested repos into submodules (creates backups of current local folders).

Included submodules (recommended)
- authorworks-user-service
- authorworks-content-service
- authorworks-storage-service
- authorworks-editor-service
- authorworks-messaging-service
- authorworks-discovery-service
- authorworks-subscription-service
- authorworks-audio-service
- authorworks-video-service
- authorworks-graphics-service
- authorworks-ui-shell
- authorworks-platform
- authorworks-docs

Excluded/legacy
- author_works/: Legacy experiments (Next.js/Leptos). Keep as-is or archive.

No redundancies
- After converting to submodules, remove any duplicate local clones and rely on the submodule content.
- Ensure each submodule points to the correct upstream origin.

