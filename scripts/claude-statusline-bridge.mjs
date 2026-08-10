#!/usr/bin/env node

import { randomUUID } from "node:crypto";
import { chmod, mkdir, open, rename } from "node:fs/promises";
import { homedir } from "node:os";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const MAX_INPUT_BYTES = 1024 * 1024;
const WINDOW_NAMES = new Set([
  "five_hour",
  "seven_day",
  "seven_day_sonnet",
  "seven_day_opus",
  "extra_usage",
]);
const NUMBER_FIELDS = new Set([
  "used_percentage",
  "remaining_percentage",
  "utilization",
]);
const DATE_FIELDS = new Set(["resets_at", "reset_at"]);
const MAX_MODEL_WINDOWS = 32;
const MODEL_KEY = /^[\p{L}\p{N} ._+-]{1,128}$/u;

export function defaultSnapshotPath(env = process.env) {
  if (env.SAGEWATCH_CLAUDE_SNAPSHOT_PATH) {
    return resolve(env.SAGEWATCH_CLAUDE_SNAPSHOT_PATH);
  }
  const dataHome = env.XDG_DATA_HOME || join(env.HOME || homedir(), ".local", "share");
  return join(dataHome, "sagewatch", "ingest", "claude-statusline.json");
}

function percentage(value) {
  return typeof value === "number" && Number.isFinite(value)
    ? Math.min(100, Math.max(0, value))
    : undefined;
}

function isoTimestamp(value) {
  if (typeof value !== "string" && typeof value !== "number") return undefined;
  const date = new Date(typeof value === "number" && Math.abs(value) < 1e12 ? value * 1_000 : value);
  return Number.isNaN(date.valueOf()) ? undefined : date.toISOString();
}

function sanitizeWindow(value) {
  if (!value || typeof value !== "object" || Array.isArray(value)) return undefined;
  const clean = {};
  for (const field of NUMBER_FIELDS) {
    const normalized = percentage(value[field]);
    if (normalized !== undefined) clean[field] = normalized;
  }
  for (const field of DATE_FIELDS) {
    const normalized = isoTimestamp(value[field]);
    if (normalized !== undefined) clean[field] = normalized;
  }
  return Object.keys(clean).length ? clean : undefined;
}

function shortPlan(value) {
  if (typeof value !== "string") return undefined;
  const normalized = value.trim();
  return normalized && normalized.length <= 64 && /^[\p{L}\p{N} ._+-]+$/u.test(normalized)
    ? normalized
    : undefined;
}

export function sanitizeStatusline(input, now = new Date(), configuredPlan) {
  if (!input || typeof input !== "object" || Array.isArray(input)) {
    throw new Error("status-line input must be a JSON object");
  }
  if (!input.rate_limits || typeof input.rate_limits !== "object" || Array.isArray(input.rate_limits)) {
    throw new Error("status-line input does not contain rate_limits");
  }

  const rateLimits = {};
  for (const name of WINDOW_NAMES) {
    const window = sanitizeWindow(input.rate_limits[name]);
    if (window) rateLimits[name] = window;
  }
  if (input.rate_limits.models && typeof input.rate_limits.models === "object" && !Array.isArray(input.rate_limits.models)) {
    const models = {};
    for (const [name, value] of Object.entries(input.rate_limits.models).slice(0, MAX_MODEL_WINDOWS)) {
      if (!MODEL_KEY.test(name)) continue;
      const window = sanitizeWindow(value);
      if (window) models[name] = window;
    }
    if (Object.keys(models).length) rateLimits.models = models;
  }
  if (!Object.keys(rateLimits).length) {
    throw new Error("rate_limits contains no supported windows");
  }

  const snapshot = {
    schema_version: 1,
    observed_at: now.toISOString(),
    rate_limits: rateLimits,
  };
  const plan = shortPlan(input.plan) ?? shortPlan(input.subscription?.plan) ?? shortPlan(configuredPlan);
  if (plan) snapshot.plan = plan;
  return snapshot;
}

export async function writeSnapshot(path, snapshot) {
  const directory = dirname(path);
  await mkdir(directory, { recursive: true, mode: 0o700 });
  await chmod(directory, 0o700);
  const temporary = join(directory, `.${randomUUID()}.tmp`);
  const handle = await open(temporary, "wx", 0o600);
  try {
    await handle.writeFile(`${JSON.stringify(snapshot)}\n`, "utf8");
    await handle.sync();
  } finally {
    await handle.close();
  }
  await chmod(temporary, 0o600);
  await rename(temporary, path);
  await chmod(path, 0o600);
}

function parseArguments(argv) {
  const marker = argv.indexOf("--passthrough");
  if (marker === -1) return [];
  const command = argv.slice(marker + 1);
  if (!command.length || !isAbsolute(command[0])) {
    throw new Error("--passthrough requires an absolute executable path");
  }
  return command;
}

async function readStdin() {
  const chunks = [];
  let size = 0;
  for await (const chunk of process.stdin) {
    size += chunk.length;
    if (size > MAX_INPUT_BYTES) throw new Error("status-line input exceeds 1 MiB");
    chunks.push(chunk);
  }
  return Buffer.concat(chunks).toString("utf8");
}

export async function main(argv = process.argv.slice(2)) {
  const passthrough = parseArguments(argv);
  const raw = await readStdin();
  const snapshot = sanitizeStatusline(JSON.parse(raw), new Date(), process.env.SAGEWATCH_CLAUDE_PLAN);
  await writeSnapshot(defaultSnapshotPath(), snapshot);

  if (passthrough.length) {
    const result = spawnSync(passthrough[0], passthrough.slice(1), {
      input: raw,
      encoding: "utf8",
      shell: false,
      maxBuffer: MAX_INPUT_BYTES,
    });
    if (result.error) throw new Error("configured status-line command failed to start");
    if (result.stdout) process.stdout.write(result.stdout);
    if (result.stderr) process.stderr.write(result.stderr);
    if (result.status !== 0) process.exitCode = result.status ?? 1;
  }
}

if (resolve(process.argv[1] || "") === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    process.stderr.write(`sagewatch bridge: ${error instanceof Error ? error.message : "failed"}\n`);
    process.exitCode = 1;
  });
}
