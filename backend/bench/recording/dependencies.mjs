import path from 'node:path';
import {createRequire} from 'node:module';
import fs from 'node:fs';
import {fileURLToPath} from 'node:url';

// Yarn links Anchor from its tools clone. Resolve peer packages from that real
// client entry so tutorial workspaces need not declare transitive dependencies.
export function recorderDependencies(workspace, anchorPackage) {
  const workspaceRequire = createRequire(path.join(workspace, 'package.json'));
  const anchorRequire = createRequire(workspaceRequire.resolve(anchorPackage));
  const toolsRoot=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../tools/recorder/package.json');
  const recorderRequire=fs.existsSync(toolsRoot)?createRequire(toolsRoot):null;
  const dependency=name=>{try{return anchorRequire(name)}catch(error){if(error.code!=='MODULE_NOT_FOUND'||!error.message.startsWith(`Cannot find module '${name}'`)||!recorderRequire)throw error;return recorderRequire(name)}};
  const web3 = dependency('@solana/web3.js');
  const bs58Module = dependency('bs58');
  const bs58 = bs58Module.default ?? bs58Module;
  const toml = dependency('toml');
  for (const name of ['VersionedTransaction', 'AddressLookupTableAccount', 'PublicKey', 'Connection']) {
    if (typeof web3[name] !== 'function') throw new Error(`Anchor client dependency lacks ${name}: ${anchorPackage}`);
  }
  if (typeof bs58.decode !== 'function' || typeof toml.parse !== 'function') {
    throw new Error(`Recorder dependencies are incompatible: ${anchorPackage}`);
  }
  return {web3, bs58, toml};
}
