export const SITE_URL = 'https://globlin.pages.dev/';
export const REPO = 'https://github.com/TOR968/Globlin';
export const RELEASES = `${REPO}/releases`;
export const LATEST_RELEASE = `${RELEASES}/latest`;
export const LATEST_RELEASE_API = 'https://api.github.com/repos/TOR968/Globlin/releases/latest';
export const ISSUES = `${REPO}/issues`;
export const ASSET_VERSION = '0.4.0';

export const TITLE = 'Globlin — Keep Your Global Packages Updated on Windows, macOS and Linux';
export const DESCRIPTION =
  'Free, open-source tray app for Windows, macOS and Linux that watches your global npm, cargo, pipx, Homebrew and winget packages and updates them in one click.';
export const SOCIAL_DESCRIPTION =
  'A tray icon for Windows, macOS and Linux (beta) that watches your global packages across fifteen package managers, plus your skills.sh agent skills, and tells you when one falls behind. One click to update.';

export function download(file: string): string {
  return `${LATEST_RELEASE}/download/${file}`;
}

export function image(name: string): string {
  return `/img/${name}?v=${ASSET_VERSION}`;
}
