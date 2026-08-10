import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const configUrl = new URL("../../src-tauri/tauri.conf.json", import.meta.url);
const stylesUrl = new URL("../../src/styles.css", import.meta.url);

test("the initial window shows both provider cards without triggering the stacked layout", async () => {
  const config = JSON.parse(await readFile(configUrl, "utf8"));
  const window = config.app.windows[0];

  const styles = await readFile(stylesUrl, "utf8");
  assert.match(styles, /@media \(max-width: 52rem\)[\s\S]*provider-card__secondary-fact \{ display: none !important;/);
  assert.match(styles, /@media \(max-width: 22\.5rem\)[\s\S]*provider-grid \{ grid-template-columns: 1fr;/);
  assert.equal(window.minWidth, 360);

  // The screenshot-driven default leaves room for the header, tabs, complete
  // provider cards, and actions without requiring an initial resize.
  assert.deepEqual(
    { width: window.width, height: window.height },
    { width: 900, height: 800 },
  );
  assert.equal(window.minHeight, 440);
});
