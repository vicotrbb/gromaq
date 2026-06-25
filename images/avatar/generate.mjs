#!/usr/bin/env node
import { generateAssetSet } from '../tools/gromaq-image-assets.mjs';

await generateAssetSet({
  kind: 'avatar',
  source: 'gromaq-avatar-with-bg.png',
  terminalColumns: 30,
  terminalRows: 15,
  terminalMode: 'quadrant-block',
});
