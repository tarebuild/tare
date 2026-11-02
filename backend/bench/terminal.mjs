import path from 'node:path';
import os from 'node:os';
import {stripVTControlCharacters} from 'node:util';
import {fileURLToPath} from 'node:url';
const backend=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
export function terminalText(value,workspace){
 let text=stripVTControlCharacters(value);
 const roots=[[process.env.TARE_HOME,'$TARE_HOME'],[workspace,'.'],[backend,'backend'],[os.homedir(),'~']].filter(([p])=>p).sort((a,b)=>b[0].length-a[0].length);
 for(const [root,replacement]of roots)for(const variant of new Set([root,root.replaceAll('\\','/')]))text=text.split(variant).join(replacement);
 return text.replaceAll('\r\n','\n');
}

export function canonicalRepository(value) {
 if (value == null) return value;
 return value.replace(/\/+$/, '').replace(/\.git$/, '').replace(/\/+$/, '');
}
