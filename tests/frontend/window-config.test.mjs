import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const configUrl = new URL("../../src-tauri/tauri.conf.json", import.meta.url);

test("the initial window shows both provider cards without triggering the stacked layout", async () => {
  const config = JSON.parse(await readFile(configUrl, "utf8"));
  const window = config.app.windows[0];

  // The provider grid stacks below 38rem (608 CSS px). Keep both the default
  // and minimum widths above that threshold so normal launch and resize states
  // retain the compact two-column widget layout.
  assert.ok(window.width > 608);
  assert.ok(window.minWidth > 608);

  // 760x500 CSS px maps to 1520x1000 physical px at 200% scaling, leaving room
  // for window chrome and a desktop panel on a typical 1920x1080 laptop.
  assert.deepEqual(
    { width: window.width, height: window.height },
    { width: 760, height: 500 },
  );
  assert.equal(window.minHeight, 480);
});
