# Security policy

## Supported versions

Only the latest release receives fixes. If you are on an older version, upgrade
before reporting.

## Reporting a vulnerability

Report it privately through GitHub's [security advisories][advisories] for this
repository. Do not open a public issue, and do not include exploit details in a
pull request.

Please include the version (`termux-settings --version`), the Termux and Android
versions, and the smallest sequence of steps that reproduces the problem. You
will get an acknowledgement within a week; if the report is valid, a fix and a
coordinated disclosure date follow.

[advisories]: https://github.com/GeneralKaos666/termux-settings/security/advisories/new

## Scope

The parts of this project that security reports should cover:

- Writes and backups under `~/.termux`, including atomic file replacement and
  the timestamped backup copies.
- Downloads of Nerd Fonts: the size cap, TLS verification, and where a
  downloaded file is allowed to land.
- Editing `termux.properties`, including preserving lines the app does not
  manage.
- Argument handling for the non-TUI commands.

## Out of scope

- The Termux app, its terminal emulator, and `termux-reload-settings`. Report
  those upstream.
- The font files themselves. They come from the upstream Nerd Fonts project;
  report artefacts there.
- Anything that requires an attacker who already has write access to your
  `~/.termux` or your user account.
