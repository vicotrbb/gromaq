#!/usr/bin/env node
import { generateAssetSet } from '../tools/gromaq-image-assets.mjs';

await generateAssetSet({
  kind: 'avatar',
  source: 'gromaq-avatar-with-bg.png',
  terminalColumns: 18,
  terminalRows: 15,
  terminalArt: 'plaque',
});
