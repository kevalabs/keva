---
trigger: always_on
---

# Version Control & Commit Standards

When generating git commit messages, branch names, or Pull Request titles, you MUST adhere to the following strict standards:

1. **Branch Naming:** `<type>/<issue-number>-<short-description>` (e.g., `feat/42-overdraft-limit`).
2. **Commit Formatting:** Use Conventional Commits. The format is strictly `<type>(<crate-name>): [#<issue-number>] <description>`.
   - Valid types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`.
   - The scope MUST be the specific crate being modified (e.g., `keva-ledger`, `keva-api`).
3. **Traceability:** Never generate a commit message without asking the user for the relevant Issue Number first if it is not already provided in the prompt.