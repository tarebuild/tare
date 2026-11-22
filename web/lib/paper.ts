// The whitepaper, as /paper presents it. Everything here comes from the paper's own build
// (whitepaper/meta.json and the PDF) through `npm run paper`, which also places the file and its first page.

import paper from './paper.json'

export const PAPER = { ...paper, href: '/paper/tare-whitepaper.pdf' }
