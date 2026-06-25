#!/usr/bin/env node
import { generateAssetSet } from '../tools/gromaq-image-assets.mjs';

await generateAssetSet({
  kind: 'avatar',
  source: 'gromaq-avatar-with-bg.png',
  terminalColumns: 20,
  terminalRows: 15,
  terminalCrop: {
    left: 0.0,
    top: 0.0,
    width: 0.58,
    height: 1.0,
  },
});
