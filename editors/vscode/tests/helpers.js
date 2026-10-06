'use strict';
const path = require('node:path');
const executable = process.env.REVOFMT_BIN || path.resolve(__dirname, '../../../target/debug/revofmt');
const defaults = { executable, indentWidth: 2, lineWidth: 80, timeoutMs: 5000 };

function document(text, eol = 1) {
  return {
    version: 1, eol, isClosed: false, uri: { toString: () => 'untitled:test.rv' },
    getText: () => text,
    positionAt(offset) {
      const before = text.slice(0, offset);
      const lines = before.split(/\r\n|\r|\n/);
      return { line: lines.length - 1, character: lines.at(-1).length };
    },
    offsetAt(position) {
      const endings = [...text.matchAll(/\r\n|\r|\n/g)];
      return (position.line === 0 ? 0 : endings[position.line - 1].index + endings[position.line - 1][0].length) + position.character;
    },
  };
}
function api(settings = {}) {
  const errors = [];
  const registrations = [];
  return {
    errors, registrations,
    EndOfLine: { LF: 1, CRLF: 2 },
    Range: class { constructor(start, end) { this.start = start; this.end = end; } },
    TextEdit: { replace: (range, newText) => ({ range, newText }) },
    workspace: { isTrusted: true, getConfiguration: () => ({ get: (name, fallback) => Object.hasOwn(settings, name) ? settings[name] : fallback }) },
    window: { showErrorMessage: (message) => { errors.push(message); } },
    languages: { registerDocumentFormattingEditProvider: (selector, provider) => {
      registrations.push({ selector, provider }); return { dispose() {} };
    } },
  };
}
function cancellation() {
  const listeners = new Set();
  return {
    isCancellationRequested: false,
    onCancellationRequested(callback) { listeners.add(callback); return { dispose: () => listeners.delete(callback) }; },
    cancel() { this.isCancellationRequested = true; for (const callback of listeners) callback(); },
  };
}
function apply(document, edits) {
  let text = document.getText();
  for (const edit of [...edits].reverse()) {
    const inserted = edit.newText.replace(/\r\n|\r|\n/g, document.eol === 2 ? '\r\n' : '\n');
    text = text.slice(0, document.offsetAt(edit.range.start)) + inserted + text.slice(document.offsetAt(edit.range.end));
  }
  return text;
}
function controlled() {
  return { ...defaults, executable: path.join(__dirname, 'fixtures', 'formatter') };
}
module.exports = { executable, defaults, document, api, cancellation, apply, controlled };
