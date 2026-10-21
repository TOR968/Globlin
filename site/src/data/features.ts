export interface Feature {
  title: string;
  body: string;
}

export const features: Feature[] = [
  {
    title: 'Fifteen package managers and your agent skills, one list',
    body: 'npm, bun, pnpm, yarn, pipx, uv, cargo, go, dotnet, gem and Homebrew are checked out of the box, plus scoop on Windows; the PowerShell Gallery, winget and choco can be switched on. The same package installed under two of them stays distinguishable — each row names its source. Agent skills installed globally with skills.sh are checked too, against the GitHub repository each one came from, and update with the same click.',
  },
  {
    title: 'Updates first, the rest folded away',
    body: 'Outdated packages lead, grouped by package manager, each group with its own Update button. Everything current folds into a tile per package manager. Chips filter by status and search narrows every list by name.',
  },
  {
    title: 'Update one, a group, or all',
    body: 'Update all runs the outdated packages as one queue with a progress bar and a result on each row; Homebrew and scoop go last, after the tools that run on them. winget and choco update too: a machine-wide winget installer and every choco update ask for an administrator through the standard Windows prompt, and a setting folds each into one prompt for the whole queue.',
  },
  {
    title: 'An ignore list that costs nothing',
    body: 'Ignored packages still show their version but are never checked, flagged or notified about. Untick one and it is re-checked immediately.',
  },
  {
    title: 'Uninstall without a dialog',
    body: 'Every row offers Uninstall — go tools included, whose binary Globlin deletes itself — which arms into a Confirm button. The two clicks are the confirmation.',
  },
  {
    title: 'Notifications only when something changes',
    body: 'A toast fires when the set of outdated packages changes, not every six hours about the same three packages.',
  },
  {
    title: 'It updates itself, signed',
    body: 'Globlin checks its own releases on the same schedule. A new build must carry a valid signature from the Globlin release key built into the app and match its SHA-256 sum, or it is discarded without touching the running app.',
  },
];
