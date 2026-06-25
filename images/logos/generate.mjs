#!/usr/bin/env node
import { generateAssetSet } from '../tools/gromaq-image-assets.mjs';

await generateAssetSet({
  kind: 'logo',
  folder: 'logos',
  source: 'gromaq-logo-with-bg.png',
  terminalColumns: 30,
  terminalRows: 12,
});
