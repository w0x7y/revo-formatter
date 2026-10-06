'use strict';
const { format } = require('./transport');
const { replacement } = require('./edits');
const { settings } = require('./settings');

function createProvider(vscode, run = format) {
  const requests = new WeakMap();
  const active = new Set();
  let disposed = false;
  return {
    async provideDocumentFormattingEdits(document, _options, token) {
      requests.get(document)?.abort();
      const operation = new AbortController();
      requests.set(document, operation);
      if (disposed || document.isClosed || token.isCancellationRequested) return [];
      const version = document.version; const eol = document.eol;
      active.add(operation);
      const cancellation = token.onCancellationRequested(() => operation.abort());
      const current = () => !disposed && !operation.signal.aborted && !token.isCancellationRequested
        && requests.get(document) === operation && !document.isClosed
        && document.version === version && document.eol === eol;
      try {
        if (!vscode.workspace.isTrusted) throw new Error('Formatting requires a trusted workspace because it executes revofmt');
        const options = settings(vscode.workspace.getConfiguration('revofmt', document.uri));
        const source = document.getText();
        const output = await run(source, options, operation.signal);
        if (!current() || !vscode.workspace.isTrusted) return [];
        const edit = replacement(source, output, eol === vscode.EndOfLine.CRLF ? '\r\n' : '\n');
        if (!edit) return [];
        const start = document.positionAt(edit.start); const end = document.positionAt(edit.end);
        // Positions cannot address the middle of an editor line-ending pair.
        if (document.offsetAt(start) !== edit.start || document.offsetAt(end) !== edit.end) {
          throw new Error('Formatter edit cannot be represented by document positions');
        }
        return [vscode.TextEdit.replace(new vscode.Range(start, end), edit.text)];
      } catch (error) {
        if (current()) vscode.window.showErrorMessage(`revofmt: ${error.message}`);
        return [];
      } finally {
        cancellation.dispose(); active.delete(operation);
        if (requests.get(document) === operation) requests.delete(document);
      }
    },
    dispose() {
      disposed = true;
      for (const operation of active) operation.abort();
      active.clear();
    },
  };
}
module.exports = { createProvider };
