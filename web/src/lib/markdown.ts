import { marked } from 'marked';
import DOMPurify from 'dompurify';
import hljs from 'highlight.js/lib/common';
import katex from 'katex';

marked.setOptions({
  gfm: true,
  breaks: true,
});

/**
 * Renders Markdown safely to sanitized HTML with AST-aware KaTeX math protection
 * (never touches fenced or inline code blocks) and Highlight.js syntax highlighting.
 */
export function renderSafeMarkdown(markdown: string): string {
  if (!markdown) return '';

  // Protect code blocks and inline code before replacing LaTeX expressions
  const codePlaceholders: string[] = [];
  const mathPlaceholders: string[] = [];

  let working = markdown.replace(/(```[\s\S]*?```|`[^`\n]+`)/g, (match) => {
    const idx = codePlaceholders.length;
    codePlaceholders.push(match);
    return `\u0000CODE_${idx}\u0000`;
  });

  // Display math $$...$$
  working = working.replace(/\$\$([\s\S]+?)\$\$/g, (_m, tex: string) => {
    const idx = mathPlaceholders.length;
    try {
      mathPlaceholders.push(
        `<div class="my-3 overflow-x-auto text-center">${katex.renderToString(tex.trim(), {
          displayMode: true,
          throwOnError: false,
          strict: false,
        })}</div>`,
      );
    } catch {
      mathPlaceholders.push(_m);
    }
    return `\u0000MATH_${idx}\u0000`;
  });

  // Inline math $...$
  working = working.replace(/(?<!\\)\$([^$\n]+?)\$/g, (_m, tex: string) => {
    const idx = mathPlaceholders.length;
    try {
      mathPlaceholders.push(
        katex.renderToString(tex.trim(), {
          displayMode: false,
          throwOnError: false,
          strict: false,
        }),
      );
    } catch {
      mathPlaceholders.push(_m);
    }
    return `\u0000MATH_${idx}\u0000`;
  });

  // Restore code blocks before marked parses
  working = working.replace(/\u0000CODE_(\d+)\u0000/g, (_m, idx: string) => {
    return codePlaceholders[Number(idx)] ?? '';
  });

  const rawHtml = marked.parse(working, { async: false }) as string;

  // Restore rendered KaTeX HTML
  const withMath = rawHtml.replace(/\u0000MATH_(\d+)\u0000/g, (_m, idx: string) => {
    return mathPlaceholders[Number(idx)] ?? '';
  });

  // Sanitize strictly with DOMPurify (allowing KaTeX MathML/SVG elements)
  return DOMPurify.sanitize(withMath, {
    ADD_TAGS: ['math', 'semantics', 'mrow', 'mi', 'mo', 'mn', 'msup', 'msub', 'mfrac', 'msqrt', 'mtext', 'annotation'],
    ADD_ATTR: ['style', 'class', 'aria-hidden', 'encoding', 'xmlns'],
  });
}

export function highlightCodeBlocks(container: HTMLElement) {
  container.querySelectorAll('pre code').forEach((el) => {
    hljs.highlightElement(el as HTMLElement);
  });
}
