#!/usr/bin/env node
import { generateAssetSet } from '../tools/gromaq-image-assets.mjs';

await generateAssetSet({
  kind: 'avatar',
  source: 'gromaq-avatar-with-bg.png',
  terminalColumns: 22,
  terminalRows: 15,
});
