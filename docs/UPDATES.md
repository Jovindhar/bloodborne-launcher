# New-version notifications

[← Documentation](README.md)

The launcher checks the releases of
[bloodborne-mac](https://github.com/bloodborne-native/bloodborne-mac/releases) against the version
built into the installed app (semantic version order; drafts ignored; launcher commits never
notify). That version is the release tag the package was built from (`BB_RELEASE_TAG`, which
bloodborne-mac's `packaging/macos.sh` passes when it builds a `v*` tag), else `CARGO_PKG_VERSION`,
which `packaging/version.sh` keeps equal to the app's.

- **A release build** checks GitHub's latest stable release (candidates ignored).
- **A release-candidate build** (`v0.9.0-rc.2`: the version files carry the release's number, the
  tag makes it a candidate) checks the 20 most recent releases, candidates included, and offers the
  highest above its own: a newer candidate, or the final release, which semantic versions order
  after its candidates. Advanced → Updates shows the candidate's full version. Builds before this
  (0.9.0 RC 1 and RC 2) report `0.9.0` and were not told about 0.9.0.
- Local and test packages (`git describe` versions) check as release builds of the launcher's
  version.

- After profile setup it checks at startup and every six hours while visible.
- A newer release opens a changelog modal (the release's GitHub notes as Markdown: headings,
  lists, emphasis; no raw HTML, images or active links) once per release tag, and a banner with
  **Changelog** and dismiss. **View release** opens the release page; download and installation
  are manual, and nothing in the app or the saves is changed.
- **Advanced → Updates:** the installed version, the check status, **Check now**, and **Check for
  new versions automatically** (`check_updates`, on by default; manual checks always work).
- Offline, rate limits and bad responses show a retryable status and never block Play. Requests
  carry no account, token or player data; 10-second timeout, 512 KiB limit.

Rust fetches GitHub's latest-release endpoint, or for a candidate the release list (reqwest,
rustls); TanStack Query shares the result (six-hour freshness). Only validated tags on the app's
release page are opened: stable ones, and for a candidate build candidates' too. Native tests cover
version order, build metadata, draft and prerelease exclusion, a candidate's newer candidates and
final release, bad responses, missing notes and link validation. Browser preview: `?update&lang=en` supplies a synthetic newer release.
