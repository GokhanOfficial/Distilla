import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { build } from 'esbuild';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const webDir = path.resolve(__dirname, '..');
const outDir = path.resolve(webDir, '..', 'assets', 'pdf');
const fontsOutDir = path.join(outDir, 'fonts');

fs.rmSync(outDir, { recursive: true, force: true });
fs.mkdirSync(fontsOutDir, { recursive: true });

// 1. Copy KaTeX offline assets (.woff2 fonts only to keep binary lean)
const katexDist = path.join(webDir, 'node_modules', 'katex', 'dist');
fs.copyFileSync(path.join(katexDist, 'katex.min.css'), path.join(outDir, 'katex.min.css'));
fs.copyFileSync(path.join(katexDist, 'katex.min.js'), path.join(outDir, 'katex.min.js'));
fs.copyFileSync(
  path.join(katexDist, 'contrib', 'auto-render.min.js'),
  path.join(outDir, 'auto-render.min.js'),
);

const katexFonts = path.join(katexDist, 'fonts');
for (const file of fs.readdirSync(katexFonts)) {
  if (file.endsWith('.woff2')) {
    fs.copyFileSync(path.join(katexFonts, file), path.join(fontsOutDir, file));
  }
}

// 2. Copy Highlight.js GitHub theme CSS
const hljsCss = path.join(webDir, 'node_modules', 'highlight.js', 'styles', 'github.min.css');
fs.copyFileSync(hljsCss, path.join(outDir, 'github.min.css'));

// 3. Bundle highlight.js/lib/common as a compact offline IIFE attaching window.hljs
await build({
  stdin: {
    contents: `import hljs from 'highlight.js/lib/common'; window.hljs = hljs;`,
    resolveDir: webDir,
    loader: 'js',
  },
  bundle: true,
  minify: true,
  format: 'iife',
  target: ['chrome110'],
  outfile: path.join(outDir, 'highlight.min.js'),
});

console.log(`Copied offline PDF assets to ${outDir}`);
