#!/usr/bin/env node
import { generateAssetSet } from '../tools/gromaq-image-assets.mjs';

await generateAssetSet({
  kind: 'avatar',
  source: 'gromaq-avatar-with-bg.png',
  // 32x15 keeps the aspect-correct avatar within the 69-column default window
  // (1280px / 18px cell - 28px padding) so every welcome stat, including the
  // 24-character "native Rust GPU terminal" tagline, renders unclipped.
  terminalColumns: 32,
  terminalRows: 15,
  terminalMode: 'quadrant-block',
  terminalCellAspect: 18 / 44,
});
