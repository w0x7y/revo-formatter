'use strict';
const { createProvider } = require('./provider');

function activateWithApi(vscode, context) {
  const provider = createProvider(vscode);
  context.subscriptions.push(provider, vscode.languages.registerDocumentFormattingEditProvider({ language: 'revo' }, provider));
}
function activate(context) { activateWithApi(require('vscode'), context); }
module.exports = { activate, activateWithApi };
