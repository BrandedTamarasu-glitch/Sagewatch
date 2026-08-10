import test from "node:test";
import assert from "node:assert/strict";
import { alertList, badge, detailsDialog, providerCard, settingsPanel } from "../../src/components/render.ts";
import { fixturePreferences, fixtureProviders } from "../../src/fixtures/status.ts";
import { deliverDesktopAlerts, reconcileStatusSnapshot, refreshMilliseconds, ThresholdAlertTracker } from "../../src/lib/alerts.ts";

test("compact cards expose trust and absolute reset information", () => {
  const html = providerCard(fixtureProviders[0], fixturePreferences, false);
  assert.match(html, /Claude/);
  assert.match(html, /remaining allowance 32%/);
  assert.match(html, /Freshness/);
  assert.match(html, /Confidence/);
  assert.match(html, /aria-haspopup="dialog"/);
  assert.doesNotMatch(html, /Resets in/);
});

test("expanded details include every allowance window and a labelled close control", () => {
  const provider = fixtureProviders[0];
  const html = detailsDialog(provider, fixturePreferences);
  provider.windows.forEach((window) => assert.match(html, new RegExp(window.label)));
  assert.match(html, /aria-label="Close Claude details"/);
  assert.match(html, /All allowance windows/);
});

test("every health state has a visible text label and icon hook", () => {
  for (const state of ["healthy", "signed_out", "unavailable", "unsupported", "source_changed", "error"]) {
    const html = badge(state, "health");
    assert.match(html, new RegExp(state));
    assert.doesNotMatch(html, /^<span[^>]*><\/span>$/);
  }
});

test("settings expose alert thresholds and explicit fallback privacy text", () => {
  const html = settingsPanel(fixturePreferences, false, true);
  assert.match(html, /name="alert_thresholds"/);
  assert.match(html, /name="autostart_enabled"[^>]*checked/);
  assert.match(html, /Start Sagewatch at login/);
  assert.match(html, /Privacy note/);
  assert.match(html, /off by default/);
});

test("settings surface autostart failures accessibly without changing the checkbox", () => {
  const html = settingsPanel(fixturePreferences, false, false, "Login integration unavailable.");
  assert.match(html, /role="status"/);
  assert.match(html, /aria-describedby="autostart-error"/);
  assert.match(html, /Login integration unavailable/);
  assert.doesNotMatch(html, /name="autostart_enabled"[^>]*checked/);
});

test("provider text is escaped before insertion", () => {
  const unsafe = { ...fixtureProviders[0], plan: "<script>bad()</script>" };
  const html = providerCard(unsafe, fixturePreferences, false);
  assert.doesNotMatch(html, /<script>/);
  assert.match(html, /&lt;script&gt;/);
});

test("refresh cadence enforces the backend minimum and honors restored seconds", () => {
  assert.equal(refreshMilliseconds(300), 300_000);
  assert.equal(refreshMilliseconds(1), 30_000);
});

test("alerts trigger only on downward crossings and dedupe until recovery", () => {
  const tracker = new ThresholdAlertTracker();
  const preferences = { ...fixturePreferences, alerts_enabled: true, alert_thresholds: [20] };
  const before = { ...fixtureProviders[0], windows: [{ ...fixtureProviders[0].windows[0], remaining_percent: 25 }] };
  const crossed = { ...before, windows: [{ ...before.windows[0], remaining_percent: 19 }] };
  assert.equal(tracker.evaluate(before, crossed, preferences).length, 1);
  assert.equal(tracker.evaluate(crossed, { ...crossed, windows: [{ ...crossed.windows[0], remaining_percent: 18 }] }, preferences).length, 0);
  const recovered = { ...crossed, windows: [{ ...crossed.windows[0], remaining_percent: 30 }] };
  tracker.evaluate(crossed, recovered, preferences);
  assert.equal(tracker.evaluate(recovered, crossed, preferences).length, 1);
});

test("exhaustion alerts render clear dismissible text", () => {
  const tracker = new ThresholdAlertTracker();
  const preferences = { ...fixturePreferences, alerts_enabled: true, alert_thresholds: [] };
  const before = { ...fixtureProviders[1], windows: [{ ...fixtureProviders[1].windows[0], remaining_percent: 1 }] };
  const exhausted = { ...before, windows: [{ ...before.windows[0], remaining_percent: 0 }] };
  const alerts = tracker.evaluate(before, exhausted, preferences);
  assert.match(alertList(alerts), /is exhausted/);
  assert.match(alertList(alerts), /Dismiss alert/);
});

test("desktop notification failures are reported without rejecting alert delivery", async () => {
  const alerts = [{ id: "claude:weekly:20", message: "Claude Weekly crossed 20% remaining." }];
  const delivered = await deliverDesktopAlerts(alerts, async () => { throw new Error("permission denied"); });
  assert.equal(delivered, false);
  assert.equal(await deliverDesktopAlerts([], async () => { throw new Error("must not run"); }), true);
});

test("a tray snapshot crossing produces one notification and remains deduped after delivery failure", async () => {
  const tracker = new ThresholdAlertTracker();
  const preferences = { ...fixturePreferences, alerts_enabled: true, alert_thresholds: [20] };
  const before = { ...fixtureProviders[0], windows: [{ ...fixtureProviders[0].windows[0], remaining_percent: 25 }] };
  const crossed = { ...before, windows: [{ ...before.windows[0], remaining_percent: 19 }] };
  const snapshot = {
    preferences,
    providers: { claude: { status: crossed, diagnostics: null, consecutive_failures: 0, next_retry_at: null, refreshing: false } },
  };
  const first = reconcileStatusSnapshot([before, fixtureProviders[1]], snapshot, tracker);
  let notifications = 0;
  const delivered = await deliverDesktopAlerts(first.alerts, async () => {
    notifications += 1;
    throw new Error("desktop notifications unavailable");
  });
  assert.equal(delivered, false);
  assert.equal(notifications, 1);
  assert.equal(first.providers.find((provider) => provider.provider === "claude").windows[0].remaining_percent, 19);

  const repeated = reconcileStatusSnapshot(first.providers, snapshot, tracker);
  assert.equal(repeated.alerts.length, 0);
  assert.equal(await deliverDesktopAlerts(repeated.alerts, async () => { notifications += 1; }), true);
  assert.equal(notifications, 1);
});
