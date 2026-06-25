import { deflateSync, inflateSync } from 'zlib';
import { dirname, join, resolve } from 'path';
import { fileURLToPath } from 'url';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'fs';

const PNG_SIGNATURE = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const OUTPUTS = {
  avatar: [
    ['avatar-transparent.png', 768],
    ['avatar-welcome.png', 320],
    ['avatar-preview.png', 512],
  ],
  logo: [
    ['logo-transparent.png', 768],
    ['logo-icon-512.png', 512],
    ['logo-icon-256.png', 256],
    ['logo-icon-128.png', 128],
  ],
};

export async function generateAssetSet({ kind, folder = kind, source, terminalColumns, terminalRows }) {
  const dir = join(ROOT, folder);
  const sourcePath = join(dir, source);
  if (!existsSync(sourcePath)) {
    throw new Error(`missing ${kind} source image: ${sourcePath}`);
  }

  const decoded = readPng(sourcePath);
  const keyed = chromaKey(decoded);
  const trimmed = trimTransparent(keyed, 8, 18);
  const outputDir = dir;
  mkdirSync(outputDir, { recursive: true });

  console.log(`Gromaq ${kind} assets`);
  console.log(`source: ${source}`);
  console.log(`input: ${decoded.width}x${decoded.height}`);
  console.log(`trimmed: ${trimmed.width}x${trimmed.height}`);

  for (const [name, size] of OUTPUTS[kind]) {
    const image = contain(trimmed, size, size);
    writePng(join(outputDir, name), image);
    console.log(`wrote ${name} ${image.width}x${image.height}`);
  }

  const terminal = terminalAnsi(trimmed, terminalColumns, terminalRows);
  const ansiName = kind === 'avatar' ? 'avatar-welcome.ansi' : 'logo-terminal.ansi';
  writeFileSync(join(outputDir, ansiName), terminal, 'utf8');
  console.log(`wrote ${ansiName} ${terminalColumns}x${terminalRows} cells`);

  const graphite = compositeOnBackground(contain(trimmed, 768, 768), [13, 17, 23, 255]);
  const graphiteName = kind === 'avatar' ? 'avatar-on-graphite.png' : 'logo-on-graphite.png';
  writePng(join(outputDir, graphiteName), graphite);
  console.log(`wrote ${graphiteName} ${graphite.width}x${graphite.height}`);
}

function readPng(path) {
  const file = readFileSync(path);
  if (!file.subarray(0, 8).equals(PNG_SIGNATURE)) {
    throw new Error(`${path} is not a PNG file`);
  }

  let offset = 8;
  let width = 0;
  let height = 0;
  let colorType = 0;
  const idat = [];

  while (offset < file.length) {
    const length = file.readUInt32BE(offset);
    const type = file.subarray(offset + 4, offset + 8).toString('ascii');
    const data = file.subarray(offset + 8, offset + 8 + length);
    offset += 12 + length;

    if (type === 'IHDR') {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      const bitDepth = data[8];
      colorType = data[9];
      const interlace = data[12];
      if (bitDepth !== 8 || ![2, 6].includes(colorType) || interlace !== 0) {
        throw new Error(`${path} must be an 8-bit non-interlaced RGB/RGBA PNG`);
      }
    } else if (type === 'IDAT') {
      idat.push(data);
    } else if (type === 'IEND') {
      break;
    }
  }

  const channels = colorType === 6 ? 4 : 3;
  const stride = width * channels;
  const raw = inflateSync(Buffer.concat(idat));
  const scanlines = Buffer.alloc(height * stride);
  let rawOffset = 0;
  let outOffset = 0;
  for (let y = 0; y < height; y++) {
    const filter = raw[rawOffset++];
    const row = raw.subarray(rawOffset, rawOffset + stride);
    rawOffset += stride;
    unfilter(row, scanlines, outOffset, stride, channels, filter);
    outOffset += stride;
  }

  const rgba = Buffer.alloc(width * height * 4);
  for (let src = 0, dst = 0; src < scanlines.length; src += channels, dst += 4) {
    rgba[dst] = scanlines[src];
    rgba[dst + 1] = scanlines[src + 1];
    rgba[dst + 2] = scanlines[src + 2];
    rgba[dst + 3] = channels === 4 ? scanlines[src + 3] : 255;
  }
  return { width, height, rgba };
}

function unfilter(row, output, outOffset, stride, bytesPerPixel, filter) {
  for (let x = 0; x < stride; x++) {
    const raw = row[x];
    const left = x >= bytesPerPixel ? output[outOffset + x - bytesPerPixel] : 0;
    const up = outOffset >= stride ? output[outOffset + x - stride] : 0;
    const upLeft = x >= bytesPerPixel && outOffset >= stride
      ? output[outOffset + x - stride - bytesPerPixel]
      : 0;
    output[outOffset + x] = (raw + predictor(filter, left, up, upLeft)) & 0xff;
  }
}

function predictor(filter, left, up, upLeft) {
  if (filter === 0) return 0;
  if (filter === 1) return left;
  if (filter === 2) return up;
  if (filter === 3) return Math.floor((left + up) / 2);
  if (filter === 4) return paeth(left, up, upLeft);
  throw new Error(`unsupported PNG filter ${filter}`);
}

function paeth(left, up, upLeft) {
  const estimate = left + up - upLeft;
  const leftDistance = Math.abs(estimate - left);
  const upDistance = Math.abs(estimate - up);
  const upLeftDistance = Math.abs(estimate - upLeft);
  if (leftDistance <= upDistance && leftDistance <= upLeftDistance) return left;
  if (upDistance <= upLeftDistance) return up;
  return upLeft;
}

function writePng(path, image) {
  const rawStride = image.width * 4;
  const raw = Buffer.alloc((rawStride + 1) * image.height);
  for (let y = 0; y < image.height; y++) {
    const src = y * rawStride;
    const dst = y * (rawStride + 1);
    raw[dst] = 0;
    image.rgba.copy(raw, dst + 1, src, src + rawStride);
  }
  const chunks = [
    chunk('IHDR', ihdr(image.width, image.height)),
    chunk('IDAT', deflateSync(raw, { level: 9 })),
    chunk('IEND', Buffer.alloc(0)),
  ];
  writeFileSync(path, Buffer.concat([PNG_SIGNATURE, ...chunks]));
}

function ihdr(width, height) {
  const data = Buffer.alloc(13);
  data.writeUInt32BE(width, 0);
  data.writeUInt32BE(height, 4);
  data[8] = 8;
  data[9] = 6;
  return data;
}

function chunk(type, data) {
  const typeBytes = Buffer.from(type, 'ascii');
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length, 0);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBytes, data])), 0);
  return Buffer.concat([length, typeBytes, data, crc]);
}

const CRC_TABLE = new Uint32Array(256).map((_, index) => {
  let value = index;
  for (let bit = 0; bit < 8; bit++) {
    value = value & 1 ? 0xedb88320 ^ (value >>> 1) : value >>> 1;
  }
  return value >>> 0;
});

function crc32(bytes) {
  let crc = 0xffffffff;
  for (const byte of bytes) {
    crc = CRC_TABLE[(crc ^ byte) & 0xff] ^ (crc >>> 8);
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function chromaKey(image) {
  const key = cornerKey(image);
  const rgba = Buffer.from(image.rgba);
  for (let i = 0; i < rgba.length; i += 4) {
    const red = rgba[i];
    const green = rgba[i + 1];
    const blue = rgba[i + 2];
    const distance = colorDistance([red, green, blue], key);
    const dominance = green - Math.max(red, blue);
    const background = dominance > 70 && distance < 95;
    const edge = dominance > 26 && distance < 165;

    if (background) {
      rgba[i + 3] = 0;
    } else if (edge) {
      const alpha = clamp(Math.round(((distance - 65) / 100) * 255), 0, 255);
      rgba[i + 3] = Math.min(rgba[i + 3], alpha);
      rgba[i + 1] = Math.min(green, Math.max(red, blue) + 18);
    }
  }
  return { width: image.width, height: image.height, rgba };
}

function cornerKey(image) {
  const samples = [
    pixel(image, 0, 0),
    pixel(image, image.width - 1, 0),
    pixel(image, 0, image.height - 1),
    pixel(image, image.width - 1, image.height - 1),
  ];
  return samples.reduce(
    (sum, sample) => [sum[0] + sample[0] / 4, sum[1] + sample[1] / 4, sum[2] + sample[2] / 4],
    [0, 0, 0],
  );
}

function trimTransparent(image, threshold, padding) {
  let left = image.width;
  let right = -1;
  let top = image.height;
  let bottom = -1;
  for (let y = 0; y < image.height; y++) {
    for (let x = 0; x < image.width; x++) {
      if (image.rgba[(y * image.width + x) * 4 + 3] <= threshold) continue;
      left = Math.min(left, x);
      right = Math.max(right, x);
      top = Math.min(top, y);
      bottom = Math.max(bottom, y);
    }
  }
  if (right < left || bottom < top) return image;
  left = Math.max(0, left - padding);
  top = Math.max(0, top - padding);
  right = Math.min(image.width - 1, right + padding);
  bottom = Math.min(image.height - 1, bottom + padding);

  const width = right - left + 1;
  const height = bottom - top + 1;
  const rgba = Buffer.alloc(width * height * 4);
  for (let y = 0; y < height; y++) {
    const src = ((top + y) * image.width + left) * 4;
    const dst = y * width * 4;
    image.rgba.copy(rgba, dst, src, src + width * 4);
  }
  return { width, height, rgba };
}

function contain(image, width, height) {
  const scale = Math.min(width / image.width, height / image.height);
  const resized = resize(image, Math.max(1, Math.round(image.width * scale)), Math.max(1, Math.round(image.height * scale)));
  const rgba = Buffer.alloc(width * height * 4);
  const left = Math.floor((width - resized.width) / 2);
  const top = Math.floor((height - resized.height) / 2);
  blit(rgba, width, resized, left, top);
  return { width, height, rgba };
}

function resize(image, width, height) {
  const rgba = Buffer.alloc(width * height * 4);
  for (let y = 0; y < height; y++) {
    const srcY = ((y + 0.5) * image.height) / height - 0.5;
    for (let x = 0; x < width; x++) {
      const srcX = ((x + 0.5) * image.width) / width - 0.5;
      sampleBilinear(image, srcX, srcY, rgba, (y * width + x) * 4);
    }
  }
  return { width, height, rgba };
}

function sampleBilinear(image, x, y, out, offset) {
  const x0 = clamp(Math.floor(x), 0, image.width - 1);
  const y0 = clamp(Math.floor(y), 0, image.height - 1);
  const x1 = clamp(x0 + 1, 0, image.width - 1);
  const y1 = clamp(y0 + 1, 0, image.height - 1);
  const tx = clamp(x - x0, 0, 1);
  const ty = clamp(y - y0, 0, 1);
  for (let c = 0; c < 4; c++) {
    const top = lerp(channel(image, x0, y0, c), channel(image, x1, y0, c), tx);
    const bottom = lerp(channel(image, x0, y1, c), channel(image, x1, y1, c), tx);
    out[offset + c] = Math.round(lerp(top, bottom, ty));
  }
}

function terminalAnsi(image, columns, rows) {
  const sampled = contain(image, columns, rows * 2);
  const lines = [];
  for (let row = 0; row < rows; row++) {
    let line = '';
    for (let col = 0; col < columns; col++) {
      const top = pixel(sampled, col, row * 2);
      const bottom = pixel(sampled, col, row * 2 + 1);
      line += terminalCell(top, bottom);
    }
    lines.push(`${line}\x1b[0m`);
  }
  return `${lines.join('\n')}\n`;
}

function terminalCell(top, bottom) {
  const topVisible = top[3] > 36;
  const bottomVisible = bottom[3] > 36;
  if (topVisible && bottomVisible) {
    return `${fg(top)}${bg(bottom)}▀`;
  }
  if (topVisible) return `${fg(top)}▀`;
  if (bottomVisible) return `${fg(bottom)}▄`;
  return ' ';
}

function compositeOnBackground(image, background) {
  const rgba = Buffer.alloc(image.rgba.length);
  for (let i = 0; i < rgba.length; i += 4) {
    const alpha = image.rgba[i + 3] / 255;
    for (let c = 0; c < 3; c++) {
      rgba[i + c] = Math.round(lerp(background[c], image.rgba[i + c], alpha));
    }
    rgba[i + 3] = 255;
  }
  return { width: image.width, height: image.height, rgba };
}

function blit(target, targetWidth, source, left, top) {
  for (let y = 0; y < source.height; y++) {
    for (let x = 0; x < source.width; x++) {
      const src = (y * source.width + x) * 4;
      const dst = ((top + y) * targetWidth + left + x) * 4;
      source.rgba.copy(target, dst, src, src + 4);
    }
  }
}

function pixel(image, x, y) {
  const offset = (y * image.width + x) * 4;
  return [
    image.rgba[offset],
    image.rgba[offset + 1],
    image.rgba[offset + 2],
    image.rgba[offset + 3],
  ];
}

function channel(image, x, y, channelIndex) {
  return image.rgba[(y * image.width + x) * 4 + channelIndex];
}

function colorDistance(a, b) {
  return Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]);
}

function fg([red, green, blue]) {
  return `\x1b[38;2;${red};${green};${blue}m`;
}

function bg([red, green, blue]) {
  return `\x1b[48;2;${red};${green};${blue}m`;
}

function lerp(a, b, t) {
  return a + (b - a) * t;
}

function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}
