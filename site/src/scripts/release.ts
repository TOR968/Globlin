import { LATEST_RELEASE_API, download } from '@/data/site';

type Os = 'windows' | 'macos' | 'linux';

const OSES: Os[] = ['windows', 'macos', 'linux'];

interface Release {
  tag_name?: string;
}

async function showLatestVersion(): Promise<void> {
  const response = await fetch(LATEST_RELEASE_API, {
    headers: { Accept: 'application/vnd.github+json' },
  });
  if (!response.ok) {
    return;
  }
  const release: Release = await response.json();
  if (!release.tag_name) {
    return;
  }
  const tag = release.tag_name;
  document.querySelectorAll('[data-version]').forEach((node) => {
    node.textContent = tag;
  });
}

function osButton(os: Os): HTMLAnchorElement | null {
  return document.querySelector<HTMLAnchorElement>(`.os-buttons [data-os="${os}"]`);
}

function choose(os: Os, file: string): void {
  for (const other of OSES) {
    const node = osButton(other);
    node?.classList.toggle('button-primary', other === os);
    node?.classList.toggle('button-ghost', other !== os);
  }
  const chosen = osButton(os);
  if (chosen) {
    chosen.href = download(file);
    chosen.dataset.file = file;
  }
  const label = document.getElementById('download-file');
  if (label) {
    label.textContent = file;
  }
}

async function pickDownload(): Promise<void> {
  const data = navigator.userAgentData;
  const platform = data?.platform || navigator.platform || '';
  if (/mac/i.test(platform)) {
    choose('macos', 'globlin-macos-apple-silicon.zip');
    return;
  }
  if (/linux/i.test(platform) && !/android/i.test(platform)) {
    choose('linux', /aarch64|arm/i.test(platform) ? 'globlin-arm64.deb' : 'globlin-amd64.deb');
    return;
  }
  if (!data?.getHighEntropyValues) {
    return;
  }
  const values = await data.getHighEntropyValues(['architecture']);
  if (values.architecture === 'arm') {
    choose('windows', 'globlin-setup-arm64.exe');
  }
}

showLatestVersion().catch(() => {});
pickDownload().catch(() => {});
