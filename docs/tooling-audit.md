# Tooling audit

The mise task layer fits Heraldr's local Worktrunk workflow. Keep it as the shared command surface for local checks, CI, and releases. The useful simplifications are removing unused features and making dependencies explicit.

`test:crate` uses `cargo package --locked --allow-dirty` to archive and compile the distributable source. It performs the packaging verification needed here without invoking the publication command or warning about an already-published version. The separate optimized build checks the release profile.

- Shell suites use Bash and standard Unix utilities. Fixed strings use `grep -F` or Bash matching; regular expressions use `grep -E`. No suite requires ripgrep. `jq`, already used to construct repository rulesets, is declared in mise and used by the ruleset regression test. GitHub operations require an authenticated `gh` installation; CI's hosted runners supply it.
- Heraldr has no Homebrew formula generator, template, or provisioning flag. Those belong in the standalone-tool variant of the shared template, not the Herdr plugin variant.
- Required branch checks come from the latest completed push run of `ci.yml` on `main`. Discovering every check attached to a commit can accidentally require release-only jobs. Provisioning must not change live repository policy during tests.
- The current official checkout, artifact transfer, and crates.io authentication actions own the GitHub-specific boundaries. `gh release upload` remains one command with an explicit repository. Replacing it with [action-gh-release](https://github.com/softprops/action-gh-release) would add a dependency without removing meaningful local code.

[dist](https://github.com/axodotdev/cargo-dist) is the strongest candidate if the tool family needs more installer formats, target platforms, or generated release manifests. It owns binary packaging and can generate the distribution workflow. For Heraldr's four native builds and exact-version plugin installer, adopting it would require reconciling archive names, installer behavior, and release ownership; that is a separate migration rather than a portability fix.

[Release-plz](https://release-plz.dev/docs) automates version bumps, release PRs, registry publication, and tags. Its primary workflow differs from the deliberate local `mise run release` sequence. Keep git-cliff and the small release tasks while local confirmation remains the preferred release interface.

Before copying this pattern, parameterize package/repository names and keep plugin version synchronization separate from generic release tasks. Keep the original-event recovery task for asset-upload failures: rerunning a historical workflow does not pick up workflow fixes. Verify the template in both hosted test jobs and a real release; local checks alone cannot prove runner provisioning or publishing permissions.
