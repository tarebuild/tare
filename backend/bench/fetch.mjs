#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const here = path.dirname(fileURLToPath(import.meta.url));
const manifest = JSON.parse(fs.readFileSync(path.join(here, 'manifest.json'), 'utf8'));
for (const repository of manifest.repositories) {
  const destination = path.join(here, repository.checkout);
  if (!fs.existsSync(destination)) {
    const gitOptions = [...(process.platform === 'win32' ? ['-c', 'http.sslBackend=openssl'] : []), '-c', 'core.autocrlf=false'];
    if (repository.tag) {
      const clone = spawnSync('git', [...gitOptions, 'clone', '--depth', '1', '--branch', repository.tag, repository.repository, destination], {stdio: 'inherit'});
      if (clone.status !== 0) process.exit(clone.status || 1);
    } else {
      fs.mkdirSync(destination, {recursive: true});
      const commands = [
        ['init', destination],
        ['-C', destination, 'remote', 'add', 'origin', repository.repository],
        ['-C', destination, 'fetch', '--depth', '1', 'origin', repository.revision],
        ['-C', destination, 'checkout', '--detach', repository.revision],
      ];
      for (const args of commands) {
        const result = spawnSync('git', [...gitOptions, ...args], {stdio: 'inherit'});
        if (result.status !== 0) process.exit(result.status || 1);
      }
    }
  }
  const revision = spawnSync('git', ['-c',`safe.directory=${destination}`,'-C', destination, 'rev-parse', 'HEAD'], {encoding: 'utf8'});
  if (revision.status !== 0 || revision.stdout.trim() !== repository.revision) throw new Error(`Expected ${repository.revision} in ${destination}; existing checkout left untouched`);
  const submodules = spawnSync('git', [...(process.platform==='win32'?['-c','http.sslBackend=openssl']:[]),'-c',`safe.directory=${destination}`,'-c','core.autocrlf=false','-C', destination, 'submodule', 'update', '--init', '--recursive', '--depth', '1'], {stdio: 'inherit'});
  if (submodules.status !== 0) process.exit(submodules.status || 1);
  console.log(`${repository.anchorVersion}: ${repository.revision}`);
}
