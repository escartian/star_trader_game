// Exports a Strudel source (assets/source/music/*.strudel.js) for the game.
// See README.md in this folder.
//
//   node tools/music/render.mjs [source.strudel.js] [output] [--bars N]
//
// Two kinds of source:
// - Blocks (`window.blocks`): each block is exported to `output/<name>.ogg`
//   with its reverb tail, plus `output/manifest.json` with every block's exact
//   length and the blocks that may follow it. The game stitches them together.
// - One loop (`window.tempoMap` with BPM per bar, or else N bars at the
//   `setcps` tempo): exported to the single file `output`, with what rings past
//   its end folded back onto its start so it loops seamlessly.
//
// Every bar renders at its own tempo with its notes ringing for `TAIL_SECONDS`,
// and bars are added together at their start times. Block RMS differences are
// compressed, then a shared peak gain leaves headroom where tails overlap.
// Audio is encoded with ffmpeg.
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { openSession } from './strudel-session.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '../..');
const args = process.argv.slice(2);
const flag = (name, fallback) => {
  const index = args.indexOf(name);
  if (index < 0) return fallback;
  const [, value] = args.splice(index, 2);
  return value;
};
const loopBars = Number(flag('--bars', '32'));
const source = resolve(args[0] ?? join(root, 'assets/source/music/long-haul.strudel.js'));
const output = resolve(args[1] ?? join(root, 'assets/music/long-haul'));
/// Seconds each bar's reverb and delay are allowed to ring after it.
const TAIL_SECONDS = 9;
/// Peak level after normalization, in dBFS. Blocks leave headroom for the
/// moment one block's tail overlaps the next block's start.
const LOOP_PEAK_DB = -1;
const BLOCKS_PEAK_DB = -6.1;
// Keep arrangement contrast, but let extra parts add density rather than a
// large jump in average loudness. Per-block correction is capped at 5 dB.
const RMS_BALANCE = 0.8;
const MAX_BLOCK_GAIN_DB = 5;
const sampleRate = 48_000;

const code = readFileSync(source, 'utf8');
// Tempo in cycles per second, from the source's own `setcps(...)` line.
const tempo = code.match(/^setcps\(([\d\s./*+-]+)\)/m);
if (!tempo) throw new Error(`${source} needs a plain arithmetic setcps(...) line`);
const cps = Function(`return (${tempo[1]})`)();
if (!(cps > 0)) throw new Error(`Bad tempo ${tempo[1]}`);

/** Bar start times in seconds for these tempos, and the total length. */
function timeline(bpms) {
  const starts = bpms.map((_, bar) => bpms.slice(0, bar).reduce((sum, bpm) => sum + 240 / bpm, 0));
  return { starts, seconds: starts[bpms.length - 1] + 240 / bpms[bpms.length - 1] };
}

/** Renders bars at `bpms` into stereo `mix` (interleaved), each from its start
 *  frame; frames past `wrap` (if given) fold back onto the start. */
async function renderBars(session, bpms, mix, wrap, label, block = null) {
  const { starts } = timeline(bpms);
  for (let bar = 0; bar < bpms.length; bar += 1) {
    await session.load();
    const barCps = bpms[bar] / 240;
    const wav = await session.renderBar(bar, barCps, TAIL_SECONDS * barCps, sampleRate, block);
    const peak = addBar(mix, wav, Math.round(starts[bar] * sampleRate), wrap);
    if (process.env.DEBUG_BARS) console.log(`  ${label} bar ${bar + 1}: peak ${peak.toFixed(3)}`);
    if (peak < 1e-3) console.warn(`  ${label} bar ${bar + 1} rendered silent`);
  }
}

/** Adds a 16-bit stereo WAV into `mix` from frame `offset` (wrapping past
 *  `wrap` frames when given, else dropping what does not fit). Returns its peak. */
function addBar(mix, wav, offset, wrap) {
  if (wav.readUInt16LE(22) !== 2) throw new Error('Unexpected render format');
  const dataStart = wav.indexOf('data', 36) + 8;
  const frames = (wav.length - dataStart) / 4;
  let peak = 0;
  for (let frame = 0; frame < frames; frame += 1) {
    const at = wrap ? (offset + frame) % wrap : offset + frame;
    if (at * 2 + 1 >= mix.length) break;
    const left = wav.readInt16LE(dataStart + frame * 4) / 32768;
    const right = wav.readInt16LE(dataStart + frame * 4 + 2) / 32768;
    mix[at * 2] += left;
    mix[at * 2 + 1] += right;
    peak = Math.max(peak, Math.abs(left), Math.abs(right));
  }
  return peak;
}

const peakOf = (mix) => mix.reduce((max, sample) => Math.max(max, Math.abs(sample)), 0);

/** Active portion only: the stored tail is excluded from transition loudness. */
function rmsOf(mix, frames) {
  let energy = 0;
  for (let sample = 0; sample < frames * 2; sample += 1) energy += mix[sample] ** 2;
  return Math.sqrt(energy / (frames * 2));
}

/** Encodes interleaved stereo `mix`, multiplied by `gain`, to Ogg Vorbis. */
function encode(mix, gain, path) {
  const pcm = Buffer.alloc(44 + mix.length * 2);
  pcm.write('RIFF', 0);
  pcm.writeUInt32LE(36 + mix.length * 2, 4);
  pcm.write('WAVEfmt ', 8);
  pcm.writeUInt32LE(16, 16);
  pcm.writeUInt16LE(1, 20);
  pcm.writeUInt16LE(2, 22);
  pcm.writeUInt32LE(sampleRate, 24);
  pcm.writeUInt32LE(sampleRate * 4, 28);
  pcm.writeUInt16LE(4, 32);
  pcm.writeUInt16LE(16, 34);
  pcm.write('data', 36);
  pcm.writeUInt32LE(mix.length * 2, 40);
  mix.forEach((sample, index) => {
    pcm.writeInt16LE(Math.max(-32768, Math.min(32767, Math.round(sample * gain * 32767))), 44 + index * 2);
  });
  const scratch = mkdtempSync(join(tmpdir(), 'star-trader-music-'));
  try {
    const wav = join(scratch, 'audio.wav');
    writeFileSync(wav, pcm);
    // Vorbis keeps lengths sample-exact (MP3 pads the start and end). Prefer
    // libvorbis; ffmpeg's built-in encoder is weaker, so it gets a high bitrate.
    const encoders = execFileSync('ffmpeg', ['-hide_banner', '-encoders']).toString();
    const codec = /\blibvorbis\b/.test(encoders)
      ? ['-c:a', 'libvorbis', '-q:a', '5']
      : ['-c:a', 'vorbis', '-strict', 'experimental', '-b:a', '192k'];
    execFileSync('ffmpeg', ['-loglevel', 'error', '-y', '-i', wav, ...codec, path]);
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}

/** Checks a block graph: known names, and tempo continuity between blocks. */
function checkBlocks(blocks, start) {
  if (!blocks[start]) throw new Error(`Start block ${start} does not exist`);
  for (const [name, block] of Object.entries(blocks)) {
    if (!/^[a-z][a-z0-9_]*$/.test(name)) throw new Error(`Bad block name ${name}`);
    if (!block.bpm?.length || block.bpm.some((bpm) => !(bpm > 0))) {
      throw new Error(`Block ${name} needs a positive tempo for each bar`);
    }
    if (block.from !== block.bpm[0] || !(block.to > 0)) {
      throw new Error(`Block ${name} has inconsistent endpoints`);
    }
    const next = Object.entries(block.next ?? {});
    if (!next.length) throw new Error(`Block ${name} has no next blocks`);
    for (const [after, weight] of next) {
      if (!blocks[after]) throw new Error(`Block ${name} leads to unknown block ${after}`);
      if (!(weight > 0)) throw new Error(`Block ${name} gives ${after} no weight`);
      if (blocks[after].from !== block.to) {
        throw new Error(`Block ${name} ends at ${block.to} BPM but ${after} starts at ${blocks[after].from}`);
      }
    }
  }
}

async function exportBlocks(session, blocks, start) {
  checkBlocks(blocks, start);
  const tailFrames = Math.round(TAIL_SECONDS * sampleRate);
  const rendered = {};
  for (const [name, block] of Object.entries(blocks)) {
    const { seconds } = timeline(block.bpm);
    const frames = Math.round(seconds * sampleRate);
    const mix = new Float32Array((frames + tailFrames) * 2);
    console.log(`Rendering block ${name}: ${block.bpm.length} bars, ${block.from}-${block.to} BPM, ${seconds.toFixed(1)} s`);
    await renderBars(session, block.bpm, mix, 0, name, name);
    const rms = rmsOf(mix, frames);
    if (!(rms > 0)) throw new Error(`Block ${name} has no active audio`);
    rendered[name] = { mix, frames, rms };
  }
  const rmsValues = Object.values(rendered).map(({ rms }) => rms).sort((a, b) => a - b);
  const targetRms = rmsValues[Math.floor(rmsValues.length / 2)];
  const limit = 10 ** (MAX_BLOCK_GAIN_DB / 20);
  const levelGain = Object.fromEntries(Object.entries(rendered).map(([name, { rms }]) => [
    name, Math.max(1 / limit, Math.min(limit, (targetRms / rms) ** RMS_BALANCE)),
  ]));
  const peak = Math.max(...Object.entries(rendered).map(([name, { mix }]) => peakOf(mix) * levelGain[name]));
  if (peak === 0) throw new Error('Render is silent');
  const gain = 10 ** (BLOCKS_PEAK_DB / 20) / peak;
  mkdirSync(output, { recursive: true });
  const manifest = { sample_rate: sampleRate, start, blocks: {} };
  for (const [name, { mix, frames, rms }] of Object.entries(rendered)) {
    encode(mix, gain * levelGain[name], join(output, `${name}.ogg`));
    const { from, to, next } = blocks[name];
    manifest.blocks[name] = { file: `${name}.ogg`, frames, rms: rms * levelGain[name], from_bpm: from, to_bpm: to, next };
  }
  writeFileSync(join(output, 'manifest.json'), `${JSON.stringify(manifest, null, 2)}\n`);
  const shaped = Object.entries(rendered).map(([name, { rms }]) => rms * levelGain[name]);
  const spread = (values) => (20 * Math.log10(Math.max(...values) / Math.min(...values))).toFixed(1);
  console.log(`Active RMS spread: ${spread(rmsValues)} dB before, ${spread(shaped)} dB after balancing`);
  console.log(`Wrote ${Object.keys(blocks).length} blocks and manifest.json to ${output}`);
}

async function exportLoop(session, tempoMap) {
  const bpms = tempoMap ?? Array(loopBars).fill(cps * 240);
  if (!bpms.length || bpms.some((bpm) => !(bpm > 0))) throw new Error('Bad tempoMap');
  const { seconds } = timeline(bpms);
  const frames = Math.round(seconds * sampleRate);
  const mix = new Float32Array(frames * 2);
  console.log(`Rendering ${bpms.length} bars at ${Math.min(...bpms)}-${Math.max(...bpms)} BPM, ${seconds.toFixed(1)} s…`);
  await renderBars(session, bpms, mix, frames, basename(source));
  const peak = peakOf(mix);
  if (peak === 0) throw new Error('Render is silent');
  encode(mix, 10 ** (LOOP_PEAK_DB / 20) / peak, output);
  console.log(`Wrote ${output}: ${seconds.toFixed(1)} s loop`);
}

const session = await openSession(code);
try {
  const piece = await session.load();
  if (piece.blocks) {
    await exportBlocks(session, piece.blocks, piece.start ?? Object.keys(piece.blocks)[0]);
  } else {
    await exportLoop(session, piece.tempoMap);
  }
} finally {
  await session.close();
}
if (!/\blibvorbis\b/.test(execFileSync('ffmpeg', ['-hide_banner', '-encoders']).toString())) {
  console.warn('ffmpeg has no libvorbis; used its built-in Vorbis encoder');
}
