// Lists Game::native_* functions that trait mods can't reach: neither a catalog hook/helper
// nor a method of the generated Host trait. Usage: node missing_helpers.js <horizon> <generated host .rs>
const fs = require("fs"), path = require("path");
const [horizon, hostFile] = process.argv.slice(2);
const cat = JSON.parse(fs.readFileSync(path.join(horizon, "native-hooks.json")));
const known = new Set();
for (const k of Object.keys(cat)) if (Array.isArray(cat[k])) for (const e of cat[k]) if (e.name) known.add(e.name);
const host = fs.readFileSync(hostFile, "utf8");
const hostFns = new Set([...host.matchAll(/fn ([a-z0-9_]+) ?\(/g)].map(m => m[1]));
const dir = path.join(horizon, "crates/horizon-client/src/game");
const found = new Map();
for (const f of fs.readdirSync(dir).filter(f => f.endsWith(".rs"))) {
  const s = fs.readFileSync(path.join(dir, f), "utf8");
  for (const m of s.matchAll(/pub\(super\) fn native_([a-z0-9_]+)\s*(<[^>]*>)?\(\s*(&(mut )?self)?/g)) {
    // Helpers take self; hooks are in the catalog. Keep self-taking functions only.
    if (m[3]) found.set(m[1], f);
  }
}
const missing = [...found].filter(([n]) => !known.has(n) && !hostFns.has(n) && !hostFns.has("level_" + n));
console.log(missing.length + " Game helpers not reachable from trait mods:");
for (const [n, f] of missing.sort()) console.log("  native_" + n + "  (" + f + ")");
