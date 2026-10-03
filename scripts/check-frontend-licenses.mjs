// Frontend license audit (M0 §26.1): walks node_modules and validates every
// package's declared license against the same policy as deny.toml.
// Usage: node scripts/check-frontend-licenses.mjs
import { readdirSync, readFileSync, existsSync } from "node:fs";
import { join, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const modulesDir = join(root, "node_modules");

const ALLOWED = new Set([
  "MIT",
  "Apache-2.0",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "ISC",
  "Zlib",
  "MPL-2.0",
  "CC0-1.0",
  "Unicode-3.0",
  "CDLA-Permissive-2.0",
  "OFL-1.0",
  "OFL-1.1",
  "0BSD",
  // —— 逐项评估过的扩展（同 deny.toml 注释纪律）——
  // Python-2.0: CNRI 许可，BSD 级宽松（argparse 等工具链数据包）
  "Python-2.0",
  // CC-BY-4.0: 仅数据包（caniuse-lite 浏览器兼容性数据），非代码
  "CC-BY-4.0",
  // MIT-0: MIT 的无署名变体，比 MIT 更宽松（@csstools 数据/辅助包）
  "MIT-0",
]);

/** 解析 SPDX 表达式：OR 取其一即可，AND 必须全部满足。 */
function isAllowedExpression(expression) {
  const normalized = expression.trim();
  const orParts = normalized.split(" OR ");
  if (orParts.length > 1) {
    return orParts.some(isAllowedExpression);
  }
  const andParts = normalized.split(" AND ");
  if (andParts.length > 1) {
    return andParts.every(isAllowedExpression);
  }
  return ALLOWED.has(normalized.replace(/^\(+|\)+$/g, ""));
}

function licenseNames(field) {
  // package.json license can be a string ("MIT"), an array, or an object
  // ({ type: "MIT" } legacy). Normalize to a list of identifier strings.
  if (!field) return [];
  if (typeof field === "string") return [field];
  if (Array.isArray(field)) return field.flatMap(licenseNames);
  if (typeof field === "object" && typeof field.type === "string") return [field.type];
  return [];
}

function packagesOf(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    if (entry.name.startsWith(".")) return [];
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name.startsWith("@")) return packagesOf(full);
      return [full];
    }
    return [];
  });
}

const violations = [];
const packages = packagesOf(modulesDir);
for (const pkgDir of packages) {
  const manifestPath = join(pkgDir, "package.json");
  if (!existsSync(manifestPath)) continue;
  let manifest;
  try {
    manifest = JSON.parse(readFileSync(manifestPath, "utf-8"));
  } catch {
    violations.push(`${pkgDir}: unparseable package.json`);
    continue;
  }
  const licenses = licenseNames(manifest.license);
  if (licenses.length === 0) {
    violations.push(`${manifest.name ?? pkgDir}: no license declared`);
    continue;
  }
  for (const license of licenses) {
    if (!isAllowedExpression(license)) {
      violations.push(`${manifest.name ?? pkgDir}: license "${license}" not in allowlist`);
    }
  }
}

if (violations.length > 0) {
  console.error(`license audit FAILED — ${violations.length} violation(s):`);
  for (const violation of violations) console.error(`  - ${violation}`);
  process.exit(1);
}
console.log(`license audit OK — ${packages.length} packages checked`);
