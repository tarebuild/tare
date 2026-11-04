import fs from 'node:fs';

export function testCounts(output) {
  const plain = output.replace(/\u001b\[[0-9;]*m/g, '');
  const sum = pattern => [...plain.matchAll(pattern)].reduce((total, match) => total + Number(match[1]), 0);
  const passed = sum(/(\d+) passing\b/g);
  const failed = sum(/(\d+) failing\b/g);
  return {ran: passed + failed, passed, failed};
}

export function writeReports(file, reports) {
  const temporary = `${file}.${process.pid}.tmp`;
  fs.writeFileSync(temporary, JSON.stringify({schemaVersion: '1', generatedAt: new Date().toISOString(), reports}, null, 2) + '\n');
  fs.renameSync(temporary, file);
}
