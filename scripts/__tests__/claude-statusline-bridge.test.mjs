import assert from "node:assert/strict";
import { mkdtemp, readFile, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import {
  defaultSnapshotPath,
  sanitizeStatusline,
  writeSnapshot,
} from "../claude-statusline-bridge.mjs";

test("uses the documented XDG data path", () => {
  assert.equal(
    defaultSnapshotPath({ XDG_DATA_HOME: "/data" }),
    "/data/sagewatch/ingest/claude-statusline.json",
  );
});

test("retains only supported rate-limit fields and clamps percentages", () => {
  const snapshot = sanitizeStatusline(
    {
      api_key: "must-not-survive",
      plan: "Claude Team",
      rate_limits: {
        five_hour: {
          used_percentage: 130,
          resets_at: "2026-08-10T20:00:00Z",
          secret: "must-not-survive",
        },
        unknown_private_window: { used_percentage: 2 },
      },
    },
    new Date("2026-08-10T19:00:00Z"),
  );

  assert.deepEqual(snapshot, {
    schema_version: 1,
    observed_at: "2026-08-10T19:00:00.000Z",
    rate_limits: {
      five_hour: {
        used_percentage: 100,
        resets_at: "2026-08-10T20:00:00.000Z",
      },
    },
    plan: "Claude Team",
  });
  assert.doesNotMatch(JSON.stringify(snapshot), /api_key|secret|must-not-survive/);
});

test("rejects missing or unsupported rate-limit data", () => {
  assert.throws(() => sanitizeStatusline({ token: "private" }), /does not contain rate_limits/);
  assert.throws(
    () => sanitizeStatusline({ rate_limits: { mystery: { used_percentage: 5 } } }),
    /no supported windows/,
  );
});

test("normalizes Unix-second resets and an explicit plan", () => {
  const snapshot = sanitizeStatusline(
    { rate_limits: { five_hour: { used_percentage: 5, resets_at: 1786388400 } } },
    new Date("2026-08-10T19:00:00Z"),
    "Claude Team",
  );
  assert.equal(snapshot.rate_limits.five_hour.resets_at, "2026-08-10T19:00:00.000Z");
  assert.equal(snapshot.plan, "Claude Team");
});

test("preserves bounded sanitized model-scoped windows using the shared adapter contract", async () => {
  const expected = JSON.parse(
    await readFile(new URL("../../tests/fixtures/claude-model-statusline.json", import.meta.url), "utf8"),
  );
  const snapshot = sanitizeStatusline(
    {
      plan: "Claude Team",
      rate_limits: {
        models: {
          "sonnet-5": {
            remaining_percentage: 42,
            resets_at: "2026-08-11T07:00:00Z",
            token: "must-not-survive",
          },
          "../invalid": { utilization: 9 },
        },
      },
    },
    new Date("2026-08-10T19:00:00Z"),
  );
  assert.deepEqual(snapshot, expected);
  assert.doesNotMatch(JSON.stringify(snapshot), /token|must-not-survive|\.\.\/invalid/);
});

test("writes an atomic private snapshot", async () => {
  const root = await mkdtemp(join(tmpdir(), "sagewatch-bridge-"));
  const path = join(root, "nested", "snapshot.json");
  await writeSnapshot(path, { schema_version: 1, rate_limits: {} });
  assert.deepEqual(JSON.parse(await readFile(path, "utf8")), {
    schema_version: 1,
    rate_limits: {},
  });
  if (process.platform !== "win32") {
    assert.equal((await stat(join(root, "nested"))).mode & 0o777, 0o700);
    assert.equal((await stat(path)).mode & 0o777, 0o600);
  }
});
