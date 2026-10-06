'use strict';
const { spawn } = require('node:child_process');
const { TextDecoder } = require('node:util');
const SOURCE_LIMIT = 262144;
const STDERR_LIMIT = 65536;

async function format(source, settings, signal) {
  if (signal?.aborted) throw new Error('Formatting canceled');
  // Buffer.from substitutes lone surrogates. Refuse that lossy conversion.
  if (Buffer.byteLength(source, 'utf8') > SOURCE_LIMIT) {
    throw new Error(`source exceeds ${SOURCE_LIMIT} UTF-8 bytes`);
  }
  const input = Buffer.from(source, 'utf8');
  if (input.toString('utf8') !== source) throw new Error('source contains an unpaired UTF-16 surrogate');

  return new Promise((resolve, reject) => {
    const child = spawn(settings.executable, [
      '--indent-width', String(settings.indentWidth),
      '--line-width', String(settings.lineWidth), '-',
    ], { shell: false, windowsHide: true, stdio: ['pipe', 'pipe', 'pipe'] });
    const stdout = []; const stderr = [];
    let outputBytes = 0; let errorBytes = 0; let settled = false;
    let timer;
    function finish(error, output) {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      signal?.removeEventListener('abort', cancel);
      if (error) {
        child.kill('SIGKILL');
        const diagnostic = Buffer.concat(stderr).toString('utf8').trim();
        reject(new Error(diagnostic ? `${error.message}: ${diagnostic}` : error.message));
      } else resolve(output);
    }
    function cancel() { finish(new Error('Formatting canceled')); }
    signal?.addEventListener('abort', cancel, { once: true });
    timer = setTimeout(() => finish(new Error(`Formatting timed out after ${settings.timeoutMs} ms`)), settings.timeoutMs);
    child.on('error', error => finish(error));
    child.stdout.on('data', chunk => {
      if (settled) return;
      outputBytes += chunk.length;
      if (outputBytes > SOURCE_LIMIT) finish(new Error(`stdout exceeds ${SOURCE_LIMIT} bytes`));
      else stdout.push(chunk);
    });
    child.stderr.on('data', chunk => {
      if (settled) return;
      errorBytes += chunk.length;
      if (errorBytes > STDERR_LIMIT) finish(new Error(`stderr exceeds ${STDERR_LIMIT} bytes`));
      else stderr.push(chunk);
    });
    child.stdin.on('error', error => {
      // Early parser exit can close stdin. Its exit code/stderr is authoritative.
      if (error.code !== 'EPIPE') finish(error);
    });
    child.on('close', (code, exitSignal) => {
      if (settled) return;
      if (signal?.aborted) return cancel();
      if (exitSignal || code !== 0) return finish(new Error(`Formatter exited with ${exitSignal ? `signal ${exitSignal}` : `code ${code}`}`));
      let output;
      try {
        // ignoreBOM means do not consume the BOM: it is part of the output.
        output = new TextDecoder('utf-8', { fatal: true, ignoreBOM: true }).decode(Buffer.concat(stdout));
      } catch {
        return finish(new Error('Formatter stdout is not valid UTF-8'));
      }
      finish(null, output);
    });
    if (signal?.aborted) cancel();
    else child.stdin.end(input);
  });
}
module.exports = { format };
