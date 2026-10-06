'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const { constants } = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawn } = require('node:child_process');
const { setTimeout: delay } = require('node:timers/promises');

async function main() {
  const extensionPath = path.resolve(__dirname, '../..');
  const executable = process.env.REVOFMT_BIN || path.resolve(extensionPath, '../../target/release/revofmt');
  assert.ok(path.isAbsolute(executable), 'REVOFMT_BIN must be an absolute executable path');
  await fs.access(executable, constants.X_OK);
  const directory = await fs.mkdtemp(path.join(os.tmpdir(), 'revofmt-vscode-host-'));
  const workspace = path.join(directory, 'workspace');
  const resultPath = path.join(workspace, 'result.json');
  const userData = path.join(directory, 'data');
  await fs.mkdir(path.join(userData, 'User'), { recursive: true });
  await fs.mkdir(workspace);
  await fs.mkdir(path.join(directory, 'extensions'));
  await fs.mkdir(path.join(directory, 'config'));
  await fs.writeFile(path.join(userData, 'User/settings.json'), JSON.stringify({
    'revofmt.executable': executable,
    'security.workspace.trust.enabled': false,
    'telemetry.telemetryLevel': 'off',
    'update.mode': 'none',
    'extensions.autoUpdate': false,
    'workbench.startupEditor': 'none',
  }));
  for (const suffix of ['rv', 'revo']) await fs.writeFile(path.join(workspace, `example.${suffix}`), 'let x=1');
  const env = { ...process.env, XDG_CONFIG_HOME: path.join(directory, 'config') };
  delete env.VSCODE_IPC_HOOK_CLI;
  delete env.VSCODE_PORTABLE;
  const log = await fs.open(path.join(directory, 'launcher.log'), 'w');
  let child;
  let passed = false;
  try {
    child = spawn(process.env.VSCODE_BIN || 'code', [
      '--new-window', '--skip-welcome', '--skip-release-notes',
      `--user-data-dir=${userData}`,
      `--extensions-dir=${path.join(directory, 'extensions')}`,
      `--extensionDevelopmentPath=${extensionPath}`,
      `--extensionTestsPath=${path.join(__dirname, 'suite.js')}`,
      workspace,
    ], { env, detached: process.platform !== 'win32', stdio: ['ignore', log.fd, log.fd] });
    let launchError;
    child.on('error', error => { launchError = error; });
    // Some code CLI wrappers exit before the native host. Only its result file
    // establishes completion; an early successful CLI exit is not a test pass.
    const deadline = Date.now() + 60000;
    let result;
    while (Date.now() < deadline) {
      if (launchError) throw launchError;
      if (child.exitCode !== null && child.exitCode !== 0) throw new Error(`VS Code launcher exited with code ${child.exitCode}`);
      try { result = JSON.parse(await fs.readFile(resultPath, 'utf8')); break; }
      catch (error) { if (error.code !== 'ENOENT') throw error; }
      await delay(100);
    }
    assert.ok(result, 'VS Code host did not write a result within 60 seconds');
    console.log(JSON.stringify(result, null, 2));
    assert.equal(result.success, true, result.error || 'Native host test failed');
    passed = true;
  } finally {
    // The detached group belongs only to this isolated host, never the user's
    // running window. The test harness also closes its window when run returns.
    if (child?.pid) {
      try { process.kill(process.platform === 'win32' ? child.pid : -child.pid, 'SIGTERM'); }
      catch (error) { if (error.code !== 'ESRCH') throw error; }
    }
    await log.close();
    if (passed && process.env.REVOFMT_HOST_KEEP !== '1') await fs.rm(directory, { recursive: true, force: true });
    else console.log(`Host evidence retained at ${directory}`);
  }
}

main().catch(error => { console.error(error); process.exitCode = 1; });
