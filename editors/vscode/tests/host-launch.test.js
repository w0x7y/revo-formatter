'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const { spawn } = require('node:child_process');

async function runLauncher(body, timeoutMs = 3000) {
  const directory = await fs.mkdtemp(path.join(os.tmpdir(), 'revofmt-host-launch-test-'));
  const launcher = path.join(directory, 'code');
  const pidPath = path.join(directory, 'launcher.pid');
  let child;
  let timer;
  try {
    await fs.writeFile(launcher, `#!${process.execPath}\n'use strict';\nconst fs = require('node:fs');\nfs.writeFileSync(${JSON.stringify(pidPath)}, String(process.pid));\n${body}\n`, { mode: 0o755 });
    child = spawn(process.execPath, [path.join(__dirname, 'host/launch.js')], {
      env: { ...process.env, TMPDIR: directory, VSCODE_BIN: launcher, REVOFMT_BIN: '/bin/true', REVOFMT_HOST_KEEP: '0' },
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    let stdout = '';
    let stderr = '';
    let timedOut = false;
    child.stdout.setEncoding('utf8').on('data', data => { stdout += data; });
    child.stderr.setEncoding('utf8').on('data', data => { stderr += data; });
    timer = setTimeout(() => { timedOut = true; child.kill('SIGKILL'); }, timeoutMs);
    const [code, signal] = await new Promise((resolve, reject) => {
      child.once('error', reject);
      child.once('close', (code, signal) => resolve([code, signal]));
    });
    return { code, signal, stdout, stderr, timedOut, files: await fs.readdir(directory) };
  } finally {
    clearTimeout(timer);
    child?.kill('SIGKILL');
    // A wrapper may exit while its host still owns the detached process group.
    // Also clean that group when the regression's deadline kills the runner.
    try { process.kill(-Number(await fs.readFile(pidPath, 'utf8')), 'SIGKILL'); }
    catch (error) { if (!['ENOENT', 'ESRCH'].includes(error.code)) throw error; }
    await fs.rm(directory, { recursive: true, force: true });
  }
}

test('host runner promptly reports a launcher terminated by SIGTERM', { skip: process.platform === 'win32' }, async () => {
  const result = await runLauncher("process.kill(process.pid, 'SIGTERM');");
  assert.equal(result.timedOut, false, 'host runner must report SIGTERM before the 3-second test deadline');
  assert.equal(result.code, 1);
  assert.equal(result.signal, null);
  assert.match(result.stderr, /launcher.*signal SIGTERM/);
  assert.match(result.stdout, /Host evidence retained at /);
});

test('host runner promptly reports a nonzero launcher exit', { skip: process.platform === 'win32' }, async () => {
  const result = await runLauncher('process.exit(23);');
  assert.equal(result.timedOut, false);
  assert.equal(result.code, 1);
  assert.match(result.stderr, /launcher exited with code 23/);
  assert.match(result.stdout, /Host evidence retained at /);
});

test('successful wrapper exit waits for atomic host result publication', { skip: process.platform === 'win32' }, async () => {
  const result = await runLauncher(`
const path = require('node:path');
const { spawn } = require('node:child_process');
const resultPath = path.join(process.argv.at(-1), 'result.json');
const publisher = spawn(process.execPath, ['-e', \`
  const fs = require('node:fs');
  setTimeout(() => {
    fs.writeFileSync(process.argv[1] + '.tmp', '{"success":true,"publisher":"delayed host"}');
    fs.renameSync(process.argv[1] + '.tmp', process.argv[1]);
    setInterval(() => {}, 1000);
  }, 500);
\`, resultPath], { stdio: 'ignore' });
publisher.unref();
process.exit(0);
`);
  assert.equal(result.timedOut, false);
  assert.equal(result.code, 0, result.stderr);
  assert.deepEqual(JSON.parse(result.stdout), { success: true, publisher: 'delayed host' });
  assert.equal(result.stderr, '');
  assert.equal(result.files.some(file => file.startsWith('revofmt-vscode-host-')), false, 'successful host data removed');
});

test('successful wrapper exit without a host result never passes', { skip: process.platform === 'win32' }, async () => {
  const result = await runLauncher('process.exit(0);', 1000);
  assert.ok(result.files.includes('launcher.pid'), 'controlled wrapper started');
  assert.equal(result.timedOut, true, 'runner must keep waiting for its host result');
  assert.equal(result.signal, 'SIGKILL');
  assert.equal(result.stdout, '');
  assert.equal(result.stderr, '');
});
