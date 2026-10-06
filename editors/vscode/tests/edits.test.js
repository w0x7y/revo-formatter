'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { replacement } = require('../src/edits');

test('returns one minimal replacement and none for unchanged or empty source', () => {
  assert.deepEqual(replacement('let x=1\n', 'let x = 1\n', '\n'), { start: 5, end: 6, text: ' = ' });
  assert.equal(replacement('', '', '\n'), null);
  assert.equal(replacement('let x = 1\n', 'let x = 1\n', '\n'), null);
});
test('does not split surrogate pairs with a shared high surrogate', () => {
  assert.deepEqual(replacement('a😀z', 'a😁z', '\n'), { start: 1, end: 3, text: '😁' });
});
test('does not split CRLF when only one half matches', () => {
  assert.throws(() => replacement('a\r\nb', 'a\nb', '\r\n'), /line ending|represent/);
});
test('accepts CRLF output exactly and rejects normalization of inserted opaque CRLF', () => {
  assert.deepEqual(replacement('let x=1\r\n', 'let x = 1\r\n', '\r\n'), { start: 5, end: 6, text: ' = ' });
  assert.throws(() => replacement("let x='a\r\nb'\nlet y=2\n", "let x = 'a\r\nb'\nlet y = 2\n", '\n'), /line ending|represent/);
});
test('retains opaque line endings outside the replacement without globally rewriting them', () => {
  assert.deepEqual(replacement("let x='a\r\nb'\nlet y=2\n", "let x='a\r\nb'\nlet y = 2\n", '\n'), { start: 18, end: 19, text: ' = ' });
});
