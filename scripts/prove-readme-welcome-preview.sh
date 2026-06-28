#!/bin/sh
set -eu

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
proof_dir="${GROMAQ_README_WELCOME_PREVIEW_PROOF_DIR:-${root}/target/readme-welcome-preview-proof}"
ppm_path="${proof_dir}/gromaq-welcome-preview.ppm"
png_path="${proof_dir}/gromaq-welcome-preview.png"
readme_png="${root}/images/screenshots/gromaq-welcome-preview.png"
summary_path="${proof_dir}/summary.txt"

if ! command -v python3 >/dev/null 2>&1; then
  printf '%s\n' "error: python3 is required to compare README welcome preview pixels" >&2
  exit 1
fi

mkdir -p "${proof_dir}"
cd "${root}"

GROMAQ_WELCOME_PREVIEW_PROOF_DIR="${proof_dir}" \
  GROMAQ_WELCOME_PREVIEW_PPM="${ppm_path}" \
  GROMAQ_WELCOME_PREVIEW_PNG="${png_path}" \
  scripts/prove-welcome-preview.sh

python3 - "${ppm_path}" "${readme_png}" <<'PY'
import struct
import sys
import zlib


def read_ppm(path):
    data = open(path, "rb").read()
    offset = 0

    def token():
        nonlocal offset
        while offset < len(data) and data[offset] in b" \t\r\n":
            offset += 1
        if offset < len(data) and data[offset] == ord("#"):
            while offset < len(data) and data[offset] not in b"\r\n":
                offset += 1
            return token()
        start = offset
        while offset < len(data) and data[offset] not in b" \t\r\n":
            offset += 1
        return data[start:offset].decode("ascii")

    magic = token()
    if magic != "P6":
        raise SystemExit(f"{path} is not a binary PPM")
    width = int(token())
    height = int(token())
    max_value = int(token())
    if max_value != 255:
        raise SystemExit(f"{path} has unsupported PPM max value {max_value}")
    if offset < len(data) and data[offset] in b" \t\r\n":
        offset += 1
    pixels = data[offset:]
    expected = width * height * 3
    if len(pixels) != expected:
        raise SystemExit(f"{path} has {len(pixels)} RGB bytes, expected {expected}")
    return width, height, pixels


def paeth(left, up, up_left):
    estimate = left + up - up_left
    left_distance = abs(estimate - left)
    up_distance = abs(estimate - up)
    up_left_distance = abs(estimate - up_left)
    if left_distance <= up_distance and left_distance <= up_left_distance:
        return left
    if up_distance <= up_left_distance:
        return up
    return up_left


def read_png_rgb(path):
    data = open(path, "rb").read()
    if not data.startswith(b"\x89PNG\r\n\x1a\n"):
        raise SystemExit(f"{path} is not a PNG")

    offset = 8
    width = height = bit_depth = color_type = None
    idat = bytearray()
    while offset < len(data):
        length = struct.unpack(">I", data[offset : offset + 4])[0]
        chunk_type = data[offset + 4 : offset + 8]
        chunk_data = data[offset + 8 : offset + 8 + length]
        offset += 12 + length
        if chunk_type == b"IHDR":
            width, height, bit_depth, color_type, compression, png_filter, interlace = struct.unpack(
                ">IIBBBBB", chunk_data
            )
            if bit_depth != 8 or color_type not in (2, 6):
                raise SystemExit(
                    f"{path} must be 8-bit RGB/RGBA, got bit depth {bit_depth}, color type {color_type}"
                )
            if compression != 0 or png_filter != 0 or interlace != 0:
                raise SystemExit(f"{path} uses unsupported PNG encoding options")
        elif chunk_type == b"IDAT":
            idat.extend(chunk_data)
        elif chunk_type == b"IEND":
            break

    if width is None or height is None:
        raise SystemExit(f"{path} is missing IHDR")

    bytes_per_pixel = 4 if color_type == 6 else 3
    stride = width * bytes_per_pixel
    inflated = zlib.decompress(bytes(idat))
    rows = []
    source_offset = 0
    previous = bytearray(stride)
    for _ in range(height):
        filter_type = inflated[source_offset]
        source_offset += 1
        raw = bytearray(inflated[source_offset : source_offset + stride])
        source_offset += stride
        recon = bytearray(stride)
        for index, value in enumerate(raw):
            left = recon[index - bytes_per_pixel] if index >= bytes_per_pixel else 0
            up = previous[index]
            up_left = previous[index - bytes_per_pixel] if index >= bytes_per_pixel else 0
            if filter_type == 0:
                recon[index] = value
            elif filter_type == 1:
                recon[index] = (value + left) & 0xFF
            elif filter_type == 2:
                recon[index] = (value + up) & 0xFF
            elif filter_type == 3:
                recon[index] = (value + ((left + up) // 2)) & 0xFF
            elif filter_type == 4:
                recon[index] = (value + paeth(left, up, up_left)) & 0xFF
            else:
                raise SystemExit(f"{path} uses unsupported PNG filter {filter_type}")
        rows.append(recon)
        previous = recon

    rgb = bytearray(width * height * 3)
    target = 0
    for row in rows:
        for source in range(0, len(row), bytes_per_pixel):
            rgb[target : target + 3] = row[source : source + 3]
            target += 3
    return width, height, bytes(rgb)


ppm_path, png_path = sys.argv[1], sys.argv[2]
ppm_width, ppm_height, ppm_pixels = read_ppm(ppm_path)
png_width, png_height, png_pixels = read_png_rgb(png_path)
if (ppm_width, ppm_height) != (png_width, png_height):
    raise SystemExit(
        f"README welcome preview dimensions {png_width}x{png_height} do not match generated {ppm_width}x{ppm_height}"
    )
if ppm_pixels != png_pixels:
    for index, (expected, actual) in enumerate(zip(ppm_pixels, png_pixels)):
        if expected != actual:
            pixel = index // 3
            channel = index % 3
            x = pixel % ppm_width
            y = pixel // ppm_width
            raise SystemExit(
                f"README welcome preview pixel mismatch at {x},{y} channel {channel}: committed {actual}, generated {expected}"
            )
    raise SystemExit("README welcome preview pixel lengths differ")
print(f"README welcome preview pixels: ok ({ppm_width}x{ppm_height})")
PY

{
  printf '%s\n' "README welcome preview proof: ok"
  printf '%s\n' "Committed PNG: ${readme_png}"
  printf '%s\n' "Generated PPM: ${ppm_path}"
  if [ -s "${png_path}" ]; then
    printf '%s\n' "Generated PNG: ${png_path}"
  fi
  printf '%s\n' "Welcome proof log: ${proof_dir}/welcome-preview.log"
} | tee "${summary_path}"
