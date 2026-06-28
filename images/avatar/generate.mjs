#!/usr/bin/env node
import { generateAssetSet } from '../tools/gromaq-image-assets.mjs';

await generateAssetSet({
  kind: 'avatar',
  source: 'gromaq-avatar-with-bg.png',
  // 36x17 keeps the aspect-correct avatar within the 69-column default window
  // (1280px / 18px cell - 28px padding) so every welcome stat, including the
  // 24-character "native Rust GPU terminal" tagline, renders unclipped.
  terminalColumns: 36,
  terminalRows: 17,
  terminalMode: 'braille',
  terminalCellAspect: 18 / 44,
  check: process.argv.includes('--check'),
});
