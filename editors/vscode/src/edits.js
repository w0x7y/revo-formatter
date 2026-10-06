'use strict';

function splitsCharacter(text, offset) {
  const left = text.charCodeAt(offset - 1); const right = text.charCodeAt(offset);
  return (left === 13 && right === 10)
    || (left >= 0xD800 && left <= 0xDBFF && right >= 0xDC00 && right <= 0xDFFF);
}

function replacement(source, output, eol) {
  if (source === output) return null;
  let start = 0;
  while (start < source.length && start < output.length && source[start] === output[start]) start++;
  while (splitsCharacter(source, start) || splitsCharacter(output, start)) start--;
  let end = source.length; let outputEnd = output.length;
  while (end > start && outputEnd > start && source[end - 1] === output[outputEnd - 1]) {
    end--; outputEnd--;
  }
  while (splitsCharacter(source, end) || splitsCharacter(output, outputEnd)) {
    end++; outputEnd++;
  }
  const text = output.slice(start, outputEnd);
  // VS Code normalizes inserted line breaks to the document EOL. Only the
  // replacement is normalized here; common prefix/suffix bytes stay opaque.
  const inserted = text.replace(/\r\n|\r|\n/g, eol);
  if (source.slice(0, start) + inserted + source.slice(end) !== output) {
    throw new Error('Formatter output cannot be represented without changing line endings; use a buffer with compatible endings');
  }
  return { start, end, text };
}
module.exports = { replacement };
