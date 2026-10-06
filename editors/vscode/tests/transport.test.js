'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { format } = require('../src/transport');
const { defaults, controlled } = require('./helpers');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

test('formats stdin with the real CLI and reaches a byte-identical fixed point', async () => {
  const output = await format('let x=1', defaults);
  assert.equal(output, 'let x = 1\n');
  assert.equal(await format(output, defaults), output);
});
test('real CLI preserves CRLF inside an opaque multiline literal', async () => {
  const output = await format("let x='first\r\nsecond'\n", defaults);
  assert.equal(output, "let x = 'first\r\nsecond'\n");
  assert.equal(await format(output, defaults), output);
});
test('real CLI handles empty input and rejects syntax without output', async () => {
  assert.equal(await format('', defaults), '');
  await assert.rejects(format('let x=', defaults), /exited.*2/);
});
test('spawns an executable directly with layout options and explicit stdin argument', async () => {
  assert.equal(await format('args', { ...controlled(), indentWidth: 4, lineWidth: 24 }), '["--indent-width","4","--line-width","24","-"]');
});
test('missing executable rejects usefully', async () => {
  await assert.rejects(format('x', { ...defaults, executable: '/missing/revofmt' }), /ENOENT/);
});
for (const [source, message] of [['overflow', /stdout.*262144/], ['stderr-overflow', /stderr.*65536/], ['invalid-utf8', /UTF-8/], ['fail', /fixture syntax error/]]) {
  test(`rejects ${source} instead of returning its stdout`, async () => {
    await assert.rejects(format(source, controlled()), message);
  });
}
test('decodes a BOM as source bytes rather than stripping it', async () => {
  assert.equal(await format('bom', controlled()), '\uFEFFx');
});
test('rejects oversized source and unpaired surrogates before launching', async () => {
  const missing = { ...defaults, executable: '/missing/revofmt' };
  await assert.rejects(format('é'.repeat(131073), missing), /source.*262144/);
  await assert.rejects(format('x\ud800', missing), /surrogate|UTF-16/);
  await assert.rejects(format('\udc00x', missing), /surrogate|UTF-16/);
});
test('times out and kills an unresponsive subprocess', async () => {
  await assert.rejects(format('hang', { ...controlled(), timeoutMs: 100 }), /timed out/);
});
test('rejects before spawn when canceled and interrupts a running subprocess', async () => {
  const first = new AbortController(); first.abort();
  await assert.rejects(format('x', defaults, first.signal), /canceled/);
  const second = new AbortController();
  const pending = format('hang', controlled(), second.signal);
  setTimeout(() => second.abort(), 100);
  await assert.rejects(pending, /canceled/);
});
test('rejects signal termination even when a subprocess already produced stdout', async () => {
  await assert.rejects(format('signal', controlled()), /signal SIGTERM/);
});
test('accepts exact byte bounds for source, stdout and stderr', async () => {
  assert.equal((await format('source-limit', controlled())).length, 262144);
  assert.equal(await format('stderr-limit', controlled()), 'ok');
  const source = 'é'.repeat(131072);
  assert.equal(await format(source, controlled()), source);
});
test('timeout and cancellation terminate the direct subprocess', async () => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'revofmt-vscode-'));
  try {
    for (const cancel of [false, true]) {
      const pidPath = path.join(directory, cancel ? 'cancel.pid' : 'timeout.pid');
      const controller = new AbortController();
      const pending = format(`hang:${pidPath}`, { ...controlled(), timeoutMs: 1000 }, controller.signal);
      const rejection = assert.rejects(pending, cancel ? /canceled/ : /timed out/);
      for (let attempts = 0; !fs.existsSync(pidPath) && attempts < 100; attempts++) {
        await new Promise(done => setTimeout(done, 5));
      }
      assert.ok(fs.existsSync(pidPath), 'fixture started');
      const pid = Number(fs.readFileSync(pidPath, 'utf8'));
      if (cancel) controller.abort();
      await rejection;
      // Rejection is immediate; wait briefly for the OS close/reap event.
      let alive = true;
      for (let attempts = 0; alive && attempts < 100; attempts++) {
        try { process.kill(pid, 0); await new Promise(done => setTimeout(done, 5)); }
        catch (error) { assert.equal(error.code, 'ESRCH'); alive = false; }
      }
      assert.equal(alive, false, 'formatter child terminated');
    }
  } finally { fs.rmSync(directory, { recursive: true, force: true }); }
});
