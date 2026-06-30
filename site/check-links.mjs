import { existsSync, readFileSync } from "node:fs";
import { dirname, join, normalize } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(fileURLToPath(import.meta.url));
const htmlPath = join(root, "index.html");
const html = readFileSync(htmlPath, "utf8");
const attributePattern = /\b(?:href|src|poster)=["']([^"']+)["']/g;
const failures = [];

for (const match of html.matchAll(attributePattern)) {
  const value = match[1];
  if (
    value.startsWith("#") ||
    value.startsWith("http://") ||
    value.startsWith("https://") ||
    value.startsWith("mailto:")
  ) {
    continue;
  }

  if (value.startsWith("../")) {
    failures.push(`Escaping deploy artifact: ${value}`);
    continue;
  }

  const withoutAnchor = value.split("#")[0];
  if (!withoutAnchor) continue;

  const localPath = normalize(join(root, withoutAnchor));
  if (!localPath.startsWith(root)) {
    failures.push(`Escaping deploy artifact: ${value}`);
    continue;
  }

  if (!existsSync(localPath)) {
    failures.push(`Missing local asset: ${value}`);
  }
}

if (failures.length > 0) {
  console.error(failures.join("\n"));
  process.exit(1);
}

console.log("Pages link check: ok");
