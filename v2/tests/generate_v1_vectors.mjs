// Run with Node >= 22.18 (built-in TypeScript stripping), no npm dependencies.
// The oracle imports the actual untouched V1 library; never the Rust port.
import { epochToDate, epochToString } from '../../src/lib/epoch.ts';
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

if (process.argv[2] === '--worker') {
  const inputs = JSON.parse(readFileSync(0, 'utf8'));
  const vectors = inputs.map(input => {
    const n = Number(input);
    const bits = Buffer.alloc(8); bits.writeDoubleBE(n);
    if (Number.isNaN(n)) return { input, error: 'No number in the clipboard.' };
    const date = epochToDate(n);
    const text = epochToString(n);
    const milliseconds = Number.isNaN(date.getTime()) ? null : date.getTime();
    let offsetSeconds = 0;
    if (milliseconds !== null) {
      // Capture second-resolution historical offsets as well as getTimezoneOffset.
      const localAsUtc = new Date(0);
      localAsUtc.setUTCFullYear(date.getFullYear(), date.getMonth(), date.getDate());
      localAsUtc.setUTCHours(date.getHours(), date.getMinutes(), date.getSeconds(), date.getMilliseconds());
      offsetSeconds = (localAsUtc.getTime() - milliseconds) / 1000;
    }
    return { input, bits: bits.toString('hex'), milliseconds, text,
      numericText: text.replace(/ \([^)]*\)$/, ''), offsetSeconds };
  });
  process.stdout.write(JSON.stringify(vectors));
} else {
  const edges = ['', ' ', '\t\n\r', '\uFEFF', '\u00a0\u2028\u3000', '\u0085', '\u180e',
    '0', '-0', '+0', '1', '-1', '1.9999', '-1.9999', '-.0009', '.0009', '1e3', '.5', '5.',
    '2147483647', '2147483647.9999', '2147483648', '2147483648.9999', '2147483649',
    '2208988800', '2208988800000', '1609459200', '1609459200000', '-2147483648',
    '8640000000000000', '8640000000000001', '-8640000000000', '-8640000000000.001',
    '-8640000000000000', '8.64e15', 'Infinity', '+Infinity', '-Infinity', '1e309', '-1e309',
    '5e-324', '-5e-324', '1e-9999', '-1e-9999',
    'NaN', 'inf', '+inf', 'infinity', '0x', '0b', '0o', '-0x1', '+0x1',
    '0x10', '0XfF', '0b101', '0B11', '0o77', '0O11', '00012', '08',
    '0b102', '0o8', '0xfg', '1_000', '1n', '1 2', '12abc', '.', '+', '1e', '1e+',
    '\uFEFF 0x10 \n', '１２', '١٢', '0x20000000000001', '0x20000000000003',
    '0x' + 'f'.repeat(256), '0x' + 'f'.repeat(257), '0b' + '1'.repeat(1200)];
  // Deterministic decimal/radix fuzz, including values around double rounding.
  let seed = 0x12345678;
  const random = () => (seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0);
  for (let i=0;i<600;i++) {
    const n = (BigInt(random()) << 32n) | BigInt(random());
    const radix = [2,8,16][i%3];
    edges.push(['0b','0o','0x'][i%3]+n.toString(radix));
    edges.push(`${i%2 ? '-' : ''}${random()}.${random()}e${i%35-10}`);
  }
  const inputs = [...new Set(edges)];
  const zones = ['UTC', 'America/New_York', 'Asia/Kathmandu', 'Europe/Paris', 'Australia/Sydney', 'Asia/Almaty'];
  const instants = [0, -1, 1609459200000, 1625097600000, 1710053999999, 1710054000000,
    1730613599999,1730613600000,1709229599999,1709229600000,-2208988800000,2208988800000];
  // Negative raw input is ALWAYS seconds, even if it looks like milliseconds.
  const zoneInputs = instants.map(ms => String(ms < 2147483648 ? ms/1000 : ms));
  const run = (zone, values) => {
    const child = spawnSync(process.execPath, [fileURLToPath(import.meta.url),'--worker'], {
      env: {...process.env, TZ: zone}, input:JSON.stringify(values), encoding:'utf8' });
    if (child.status !== 0) throw new Error(child.stderr);
    return JSON.parse(child.stdout);
  };
  const hash = p => createHash('sha256').update(readFileSync(new URL(p,import.meta.url))).digest('hex');
  const fixture = { node:process.version, icu:process.versions.icu, tzdata:process.versions.tz,
    sourceCommit:'59ef35503dbf44c36326f771cf102b6276adc1da',
    sourceHashes:{'src/lib/epoch.ts':hash('../../src/lib/epoch.ts'),'src/epoch.tsx':hash('../../src/epoch.tsx')},
    utc:run('UTC',inputs), zones:zones.map(zone=>({zone,vectors:run(zone,zoneInputs)})) };
  writeFileSync(new URL('./fixtures/v1-vectors.json',import.meta.url),JSON.stringify(fixture,null,2)+'\n');
  console.log(`Wrote ${fixture.utc.length} UTC and ${zones.length*zoneInputs.length} local oracle vectors.`);
}
