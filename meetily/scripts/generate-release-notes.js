#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

const DEFAULT_LIMITATIONS = [
  'Windows x64 is the supported signed installer target for this release.',
  'Bundled local transcription uses CPU mode and may be slow on low-power laptops.',
  'Battery-aware throttling depends on OS support and may report as not detected.',
];

const DEFAULT_HARDWARE = [
  'Windows 10 or later, x64.',
  '8 GB RAM minimum; 16 GB recommended for longer meetings.',
  '4 CPU cores minimum; 6 or more recommended for smoother local transcription.',
  'At least 6 GB free disk space for the app, bundled runtime, model cache, and recordings.',
];

function normalizeVersion(version) {
  if (!version) return '0.0.0';
  return version.startsWith('v') ? version.slice(1) : version;
}

function generateReleaseNotes({ version, commit, artifacts = [] }) {
  const cleanVersion = normalizeVersion(version);
  const lines = [
    `# Meetily ${cleanVersion}`,
    '',
    `Commit: \`${commit || 'unknown'}\``,
    '',
    '## Artifacts',
    '',
  ];

  if (artifacts.length === 0) {
    lines.push('- No artifacts were supplied to the release notes generator.');
  } else {
    for (const artifact of artifacts) {
      lines.push(`- \`${artifact}\``);
    }
  }

  lines.push('', '## Minimum hardware', '');
  for (const item of DEFAULT_HARDWARE) {
    lines.push(`- ${item}`);
  }

  lines.push('', '## Known limitations', '');
  for (const item of DEFAULT_LIMITATIONS) {
    lines.push(`- ${item}`);
  }

  lines.push(
    '',
    '## Clean install notes',
    '',
    '- Installers are signed for Windows x64.',
    '- The installer includes the Python backend, faster-whisper-server runtime, and bundled base model.',
    '- No global Python, Node.js, Docker, or terminal startup script should be required after installation.',
    '- Use Settings > Diagnostics after first launch to confirm backend, whisper, model, and CPU profile health.',
    ''
  );

  return lines.join('\n');
}

function listArtifacts(artifactDir) {
  if (!artifactDir || !fs.existsSync(artifactDir)) return [];
  return fs
    .readdirSync(artifactDir)
    .filter((name) => {
      const fullPath = path.join(artifactDir, name);
      return fs.statSync(fullPath).isFile();
    })
    .sort();
}

function main(argv) {
  const [version, commit, artifactDir, outputFile = 'RELEASE_NOTES.md'] = argv;
  const notes = generateReleaseNotes({
    version,
    commit,
    artifacts: listArtifacts(artifactDir),
  });
  fs.writeFileSync(outputFile, notes, 'utf8');
  console.log(`Wrote release notes to ${outputFile}`);
}

if (require.main === module) {
  main(process.argv.slice(2));
}

module.exports = {
  generateReleaseNotes,
  listArtifacts,
};
