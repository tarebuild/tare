# Brings the whitepaper into the site from its own build: the PDF it serves, its first page as an image,
# and everything /paper says about it (whitepaper/meta.json, plus the page count and size read from the PDF).
#   python scripts/paper.py [path to the PDF; meta.json is read beside it]
import json
import shutil
import sys
from pathlib import Path

import fitz

web = Path(__file__).resolve().parent.parent
src = Path(sys.argv[1]) if len(sys.argv) > 1 else web.parent / 'whitepaper/tare-whitepaper.pdf'
meta = json.loads((src.parent / 'meta.json').read_text(encoding='utf8'))


def section(s):
    """{'number': '3' or 'A', 'title', 'page'} as /paper lists it."""
    return {'n': str(s['number']), 'title': s['title'], 'page': s.get('page')}


shutil.copyfile(src, web / 'public/paper/tare-whitepaper.pdf')
doc = fitz.open(src)
doc[0].get_pixmap(dpi=144).save(web / 'app/paper/cover.png')

abstract = meta['abstract']
paper = {
    'title': meta['title'],
    'subtitle': meta['subtitle'],
    'authors': meta['author'],
    'version': str(meta['version']),
    'date': meta['date'],
    'abstract': abstract if isinstance(abstract, list) else [p.strip() for p in abstract.split('\n\n') if p.strip()],
    'sections': [section(s) for s in meta['sections']],
    'pages': doc.page_count,
    'bytes': src.stat().st_size,
}
(web / 'lib/paper.json').write_text(json.dumps(paper, indent=2, ensure_ascii=False) + '\n', encoding='utf8')
print(f"copy  {src} -> public/paper/tare-whitepaper.pdf (v{paper['version']}, {paper['pages']} pages)")
