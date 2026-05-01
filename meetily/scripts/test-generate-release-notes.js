const assert = require('assert');
const { generateReleaseNotes } = require('./generate-release-notes');

const notes = generateReleaseNotes({
  version: '1.2.3',
  commit: 'abc1234',
  artifacts: [
    'meetily_1.2.3_x64-setup.exe',
    'meetily_1.2.3_x64_en-US.msi',
    'latest.json',
    'SHA256SUMS.txt',
  ],
});

assert(notes.includes('# Meetily 1.2.3'));
assert(notes.includes('Commit: `abc1234`'));
assert(notes.includes('meetily_1.2.3_x64-setup.exe'));
assert(notes.includes('Minimum hardware'));
assert(notes.includes('Known limitations'));
assert(notes.includes('Clean install notes'));

console.log('release notes generation test passed');
