# Security policy

## Supported versions

Astralbase has not reached a stable release. Security fixes are applied to the
latest commit on the default branch and, after publication, to the newest
`0.1.x` release.

## Reporting a vulnerability

Do not open a public issue for a suspected vulnerability. Use GitHub's private
security-advisory reporting for this repository. Include the affected commit or
version, a minimal reproducer, expected impact, and any proposed mitigation.

Astralbase processes chess positions and research artifacts. Treat parser
crashes, unbounded resource use on bounded APIs, certificate-validation bypasses,
and provenance or digest confusion as security-relevant. A bounded proof result
that can be misrepresented as an exhaustive chess result is also in scope.

The maintainer will acknowledge a complete report within seven days. No
embargo or remediation date is promised for this pre-release project.
