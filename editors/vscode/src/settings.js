'use strict';

function settings(configuration) {
  const values = {
    executable: configuration.get('executable', 'revofmt'),
    indentWidth: configuration.get('indentWidth', 2),
    lineWidth: configuration.get('lineWidth', 80),
    timeoutMs: configuration.get('timeoutMs', 5000),
  };
  if (typeof values.executable !== 'string' || !values.executable.trim() || values.executable.includes('\0')) {
    throw new Error('revofmt.executable must be a nonempty executable name or path without NUL bytes');
  }
  for (const [name, minimum, maximum] of [['indentWidth', 1, 8], ['lineWidth', 20, 240], ['timeoutMs', 1, 2147483647]]) {
    if (!Number.isInteger(values[name]) || values[name] < minimum || values[name] > maximum) {
      throw new Error(`revofmt.${name} must be an integer from ${minimum} to ${maximum}`);
    }
  }
  return values;
}
module.exports = { settings };
