// The docs, as data. Each page is a list of sections, so the article, the on-page contents and the
// search index are read from one place. Keys are the paths in lib/docs.ts.
// Sources: PLAN.md and BRIEF.md for what the product is, backend/toolchain for how the CLI behaves,
// @tare/contract for the documents, lib/bench.json for every number.

import Link from 'next/link'
import type { ReactNode } from 'react'
import { Compatibility } from '@/components/docs/Compatibility'
import { C, List, Shell, Source, Steps, Table } from '@/components/docs/kit'
import { featured } from '@/lib/bench'
import { CORPORA, IDENTICAL, INSTALL, LIMITS, REGISTRY_PROGRAM_ID, SITE_URL, TOKEN, WEIGHTS } from '@/lib/facts'
import { units } from '@/lib/format'
import { API_URL } from '@/lib/api'

export type DocSection = { id: string; title: string; body: ReactNode }
export type BannerColour = 'white' | 'green' | 'magenta' | 'yellow' | 'cyan' | 'red' | 'blue' | 'line'
export type Doc = { lead: string; banner: { t: string; c: BannerColour }[]; sections: DocSection[] }

const L = ({ href, children }: { href: string; children: ReactNode }) => (
  <Link href={href} className="link">
    {children}
  </Link>
)

const NL = String.fromCharCode(10)
const pad = (v: string | number, n: number) => String(v).padStart(n)

/** The featured benchmark program in the layout `tare weigh` prints. */
function weighOutput(): string | null {
  if (!featured) return null
  const w = featured.weighing
  const name = w.program.name ?? featured.name
  const head = `${name.padEnd(20)}${pad('gross', 11)}${pad('tare', 11)}${pad('net', 11)}${pad('inputs', 11)}`
  const lines = w.instructions.map((ix) =>
    ix.gross === null
      ? `${(ix.name ?? ix.discriminator).padEnd(20)} not measured (0 successful inputs)`
      : `${(ix.name ?? ix.discriminator).padEnd(20)}${pad(ix.gross, 11)}${pad(ix.tare, 11)}${pad(ix.net, 11)}${pad(ix.inputs, 11)}`,
  )
  const total = `${'total'.padEnd(20)}${pad(w.totals.gross, 11)}${pad(w.totals.tare, 11)}${pad(w.totals.net, 11)}${pad(w.totals.inputs, 11)}`
  return [head, ...lines, total].join(NL)
}
const WEIGH_OUTPUT = weighOutput()
const PROOF_LINE = featured ? `proof: ${featured.proof.status} (${featured.proof.comparisons.inputs} transactions)` : null

/** The featured weighing as weigh.json holds it, with its instruction list cut to two. */
function weighDocument(): string | null {
  if (!featured) return null
  const w = featured.weighing
  return JSON.stringify(
    {
      schemaVersion: w.schemaVersion,
      program: w.program,
      toolchainVersion: w.toolchainVersion,
      anchorVersion: w.anchorVersion,
      source: w.source,
      builds: w.builds,
      instructions: w.instructions.slice(0, 2),
      totals: w.totals,
    },
    null,
    2,
  )
}
const WEIGH_DOCUMENT = weighDocument()

function proofDocument(): string | null {
  if (!featured) return null
  return JSON.stringify({ proof: { ...featured.proof, firstDivergence: null } }, null, 2)
}
const PROOF_DOCUMENT = proofDocument()

// Real CLI output from backend/bench/transcripts/chain/, run on a local validator. No page states the
// attest fee's figure (PLAN.md *Decisions*: the docs read it from Config), so its value is left as <fee>; the page URL is the site's.
const ATTEST_CAPTURE = [
  '$ tare attest --host --manifest-path programs/squads_multisig_program/Cargo.toml',
  'attest fee: <fee> lamports',
  'record: AK1RjjovGLJbj5vEdzh4EtMEwdkTsRN2PYUVUASz4hpL',
  'transaction: 3T4dBCAhtYA99G512mbNTkukQyu8bG4NJoKFPpfGoBUrbuPHn4imw9cZk6K1VNemP5As336isuww2NeuCk4fPQ4k',
  `${SITE_URL}/builds/AK1RjjovGLJbj5vEdzh4EtMEwdkTsRN2PYUVUASz4hpL`,
].join(NL)
const CHECK_CAPTURE = `$ tare check AK1RjjovGLJbj5vEdzh4EtMEwdkTsRN2PYUVUASz4hpL --host
record: AK1RjjovGLJbj5vEdzh4EtMEwdkTsRN2PYUVUASz4hpL
commit: 3a39735d0c56249bf2ce4b16b3229d7be87ad55f
building squads-multisig-program (Anchor 0.32.1)
account ABI: SDK (typed AccountInfo boundary)
account input: compact SDK account views
bytecode: 23 instructions rewritten; 826176 -> 777488 bytes
IDL: explicit supplied input supplied-idl.json
squads-multisig-program: stock 791768 bytes -> tare 777488 bytes
stock hash: f228b6dfcc6eaf5d3cb0a52bce0ee11fc32c7a25af8d802cdc05eea0597b2b00 (matches)
tare hash: 14f7ff1817c5896be925ab472bf999aaef082e5c26a98ac36a2ec4254fa36493 (matches)
corpus: 488 transactions rerun
matches`
const FLAG_REFUSED_CAPTURE = `$ tare flag 7dSFXGXxgH2HUUMrC5jh8zJE286Ldmzu2iFWXPe6VmS7 --input new-idl-input.json
record: 7dSFXGXxgH2HUUMrC5jh8zJE286Ldmzu2iFWXPe6VmS7
commit: 0e115d4bd34b7a9f61cbe6ade144dae2d5d12c66
building new-idl (Anchor 0.32.3)
bytecode: 8 instructions rewritten; 267880 -> 249768 bytes
new-idl: stock 283536 bytes -> tare 249768 bytes
stock hash: bb4a41c5c8301fe42f542a1ec25574e0c9d1b05dc49b7c3666565b8bad4f6398 (matches)
tare hash: 39ed295a8e2cf7a88ec1675f646062bc4ca542349c5ff62892c16b67e274af0a (matches)
error: input does not diverge; no flag was uploaded or sent`

const FROM_BENCH = featured ? (
  <p>
    The figures are {featured.name} from <L href="/bench">the benchmark</L>.
  </p>
) : null

export const CONTENT: Record<string, Doc> = {
  /* ───────────── Get started ───────────── */

  '': {
    lead: 'Tare builds the Anchor program you already wrote into one that burns a fraction of the compute, and proves that it is the same program.',
    banner: [
      { t: 'anchor build', c: 'line' },
      { t: 'tare build', c: 'green' },
    ],
    sections: [
      {
        id: 'what',
        title: 'What Tare is',
        body: (
          <>
            <p>
              Tare is a build tool for Solana programs written in Anchor, and a public registry of what it built. It reads an existing
              Anchor workspace unchanged: no source edit, no manifest edit, no migration. Where you run <C>anchor build</C> today, you
              run <C>tare build</C>.
            </p>
            <p>
              An Anchor program&rsquo;s cost is not in your handlers. It is in what the framework generates around them: the entrypoint
              that builds every account, the validation, serialization in and out, allocation, the error paths. Tare is a second
              implementation of the Anchor language that generates the lean version of all of that from the same macros and types, and a
              bytecode stage over the linked program.
            </p>
          </>
        ),
      },
      {
        id: 'commands',
        title: 'The commands',
        body: (
          <>
            <Table
              head={['Command', 'What it does', 'Chapter']}
              rows={[
                [<C key="1">tare build</C>, 'Compiles the workspace with stock Anchor and with Tare.', <L key="l" href="/docs/build">Build</L>],
                [<C key="2">tare weigh</C>, 'Measures gross, tare and net for every instruction.', <L key="l" href="/docs/weigh">Weigh</L>],
                [<C key="3">tare prove</C>, 'Runs both builds over the same inputs and compares every result.', <L key="l" href="/docs/prove">Prove</L>],
                [<C key="4">tare attest</C>, 'Writes the proof to a record on chain.', <L key="l" href="/docs/check/attest">tare attest</L>],
                [<C key="5">tare check</C>, 'Rebuilds and reruns a record from the record alone.', <L key="l" href="/docs/check/check">tare check</L>],
                [<C key="6">tare flag</C>, 'Files an input on which the two builds differ.', <L key="l" href="/docs/check/flag">tare flag</L>],
              ]}
            />
            <p>Building, weighing and proving run on your machine and cost nothing. Attesting carries a fee in SOL.</p>
          </>
        ),
      },
      {
        id: 'path',
        title: 'From a workspace to a record',
        body: (
          <Steps>
            <li>
              <L href="/docs/install">Install</L> the <C>tare</C> binary.
            </li>
            <li>
              In your program&rsquo;s workspace, <C>tare build</C>. You get the stock program and the Tare program side by side.
            </li>
            <li>
              <C>tare weigh</C> prints the <L href="/docs/weights">three weights</L> for every instruction.
            </li>
            <li>
              <C>tare prove</C> shows the two builds behave identically on a corpus of inputs.
            </li>
            <li>
              <C>tare attest</C> publishes that proof. Anyone can run <C>tare check</C> on the record.
            </li>
            <li>
              <L href="/docs/build/deploy">Deploy</L> the Tare build as an upgrade of the same program.
            </li>
          </Steps>
        ),
      },
    ],
  },

  weights: {
    lead: 'Every instruction of a program gets three numbers in compute units. Together, per instruction, they are a weighing.',
    banner: [
      { t: 'gross', c: 'line' },
      { t: 'tare', c: 'magenta' },
      { t: 'net', c: 'green' },
    ],
    sections: [
      {
        id: 'three',
        title: 'Gross, tare, net',
        body: (
          <>
            <Table head={['Weight', 'What it is']} rows={WEIGHTS.map((w) => [<b key={w.key}>{w.label}</b>, w.says])} />
            <p>
              The names come from weighing goods: the gross weight is the goods with their container, the tare is the container, the net
              is what is left. Here the container is the framework&rsquo;s generated code.
            </p>
          </>
        ),
      },
      {
        id: 'measured',
        title: 'How a weight is measured',
        body: (
          <>
            <p>
              Both builds run the same inputs in a local SVM with the same metering. An instruction&rsquo;s gross and net are the{' '}
              <b>median</b> compute units over its successful calls, on the stock build and on the Tare build. The median is the upper
              middle observation, so every figure is a value the meter actually reported.
            </p>
            <p>
              Tare is gross minus net. A program&rsquo;s totals are the sums over its measured instructions: one call of each.
            </p>
          </>
        ),
      },
      {
        id: 'why',
        title: 'What the numbers are read against',
        body: (
          <p>
            Priority fees are charged per compute unit requested. A transaction is capped at {units(LIMITS.transactionUnits)} units and
            an account at {units(LIMITS.accountUnitsPerBlock)} per block. Net is what your users request and what a composition of your
            instructions has to fit under.
          </p>
        ),
      },
    ],
  },

  identical: {
    lead: 'The Tare build is the same program to everything that talks to it.',
    banner: IDENTICAL.slice(0, 3).map((t, i) => ({ t: t.toLowerCase(), c: (['green', 'cyan', 'white'] as const)[i]! })),
    sections: [
      {
        id: 'list',
        title: 'What is identical',
        body: (
          <Table
            head={['', 'Who relies on it']}
            rows={[
              ['Account layouts', 'Every account your program already owns on chain.'],
              ['Discriminators', 'Clients building instructions, and accounts being read back.'],
              ['The IDL', 'Generated clients, explorers, wallets. It is byte-identical.'],
              ['Error codes and messages', 'Clients that branch on an error, and anyone reading a failed transaction.'],
              ['Events', 'Indexers.'],
              ['Logs', 'Indexers and explorers, including the instruction-name lines.'],
              ['Return data', 'Programs that call yours and read what it returns.'],
            ]}
          />
        ),
      },
      {
        id: 'order',
        title: 'Order of checks',
        body: (
          <p>
            Declared-ID checks, instruction logs, argument deserialization, account extraction, constraints, the handler, return data and
            exit keep their observable order. When several constraints fail at once, the error reported is the one stock Anchor reports.
          </p>
        ),
      },
      {
        id: 'upgrade',
        title: 'What that allows',
        body: (
          <p>
            Programs on Solana upgrade in place over their existing accounts. Because layouts and discriminators are unchanged, the Tare
            build is deployed as an ordinary upgrade at the same address, with no account migration.{' '}
            <L href="/docs/build/deploy">Deploy the Tare build</L>
          </p>
        ),
      },
      {
        id: 'shown',
        title: 'How it is shown',
        body: (
          <p>
            Identity is not asserted, it is run: <C>tare prove</C> executes both builds over the same inputs and compares the results
            after every transaction. <L href="/docs/prove/comparison">What is compared</L>
          </p>
        ),
      },
    ],
  },

  install: {
    lead: 'One binary, tare, beside the Anchor and Solana tools you already have.',
    banner: [{ t: 'tare --version', c: 'white' }],
    sections: [
      {
        id: 'install',
        title: 'Install',
        body: INSTALL ? (
          <>
            <Shell>{`$ ${INSTALL}`}</Shell>
            <p>
              The script reads the latest release, downloads the prebuilt binary for your platform, checks it against its SHA-256 and
              copies it to <C>~/.tare/bin</C>. It prints <C>Installed &lt;version&gt; to &lt;folder&gt;</C>, then the line to add that
              folder to your <C>PATH</C> when it is not on it yet, then the binary&rsquo;s own version.
            </p>
            <Table
              head={['Variable', 'What it changes']}
              rows={[
                [<C key="1">TARE_VERSION</C>, 'Installs that release instead of the latest.'],
                [<C key="2">TARE_INSTALL_DIR</C>, <span key="2s">Installs into that folder instead of <C>~/.tare/bin</C>.</span>],
              ]}
            />
            <Table
              head={['System', 'Machines']}
              rows={[
                ['macOS', 'Apple silicon and Intel'],
                ['Linux with glibc', 'x86_64 and aarch64'],
                ['Windows, from Git Bash', 'x86_64'],
              ]}
            />
            <p>
              On musl-based Linux, such as Alpine, the script stops and says so. It needs <C>curl</C>, <C>tar</C>, and{' '}
              <C>sha256sum</C> or <C>shasum</C>, and it sets no environment variables.
            </p>
          </>
        ) : (
          <>
            <p>Build the binary from the toolchain workspace with Cargo:</p>
            <Shell>{`$ cargo install --path toolchain/crates/tare`}</Shell>
          </>
        ),
      },
      {
        id: 'verify',
        title: 'Check the install',
        body: (
          <>
            <Shell>{`$ tare --version`}</Shell>
            <p>
              The version printed is the toolchain version. It is written into every weighing, proof and record, so a build can be
              reproduced with the same toolchain later.
            </p>
          </>
        ),
      },
      {
        id: 'first',
        title: 'First run',
        body: (
          <>
            <p>In the folder of an Anchor program:</p>
            <Shell>{`
$ tare build
$ tare weigh
`}</Shell>
            <p>
              <L href="/docs/build">Build</L> and <L href="/docs/weigh">Weigh</L> describe what each prints and writes.
            </p>
          </>
        ),
      },
    ],
  },

  'install/requirements': {
    lead: 'What has to be on the machine before tare build runs.',
    banner: [{ t: 'requirements', c: 'line' }],
    sections: [
      {
        id: 'workspace',
        title: 'An Anchor workspace',
        body: (
          <p>
            A program on one of the <L href="/docs/compatibility/versions">pinned Anchor versions</L>. Nothing in it changes: Tare reads
            your <C>Cargo.toml</C> and source in place.
          </p>
        ),
      },
      {
        id: 'docker',
        title: 'Docker',
        body: (
          <p>
            By default both compilations run inside one <L href="/docs/build/container">pinned container image</L>, which carries Rust,
            the Solana platform tools and the SVM. With <C>--host</C> the CLI uses the tools on your machine instead: Cargo and{' '}
            <C>cargo-build-sbf</C>.
          </p>
        ),
      },
      {
        id: 'keypair',
        title: 'A keypair, for two commands',
        body: (
          <p>
            <C>tare attest</C> and <C>tare flag</C> sign a transaction, so they take a Solana keypair holding SOL. Every other command
            needs no key and no network account.
          </p>
        ),
      },
    ],
  },

  'install/environment': {
    lead: 'The variables the CLI reads, and the flags every build command takes.',
    banner: [
      { t: 'TARE_IMAGE', c: 'white' },
      { t: 'TARE_HOME', c: 'line' },
    ],
    sections: [
      {
        id: 'variables',
        title: 'Variables',
        body: (
          <Table
            head={['Variable', 'What it sets', 'Default']}
            rows={[
              [
                <C key="1">TARE_IMAGE</C>,
                'The container image both builds run in. A tag, or an immutable repository@sha256 digest.',
                <span key="d">
                  <C>tare-toolchain:&lt;version&gt;</C>, the version <C>tare --version</C> prints
                </span>,
              ],
              [
                <C key="2">TARE_HOME</C>,
                <span key="2s">
                  The folder holding Tare&rsquo;s implementation of the Anchor crates, read by <C>--host</C> runs.
                </span>,
                'The source folder the binary was built from',
              ],
            ]}
          />
        ),
      },
      {
        id: 'network',
        title: 'Variables for the registry',
        body: (
          <>
            <p>
              <C>tare attest</C>, <C>tare check</C> and <C>tare flag</C> read the registry and its API. Each variable has a flag of the
              same meaning; a flag wins over the variable. None has a default, and a command that needs one it lacks stops and names it.
            </p>
            <Table
              head={['Variable', 'Flag', 'What it is', 'Read by']}
              rows={[
                [<C key="1">TARE_RPC_URL</C>, <C key="1f">--url</C>, 'A Solana RPC endpoint on the cluster the registry is on.', 'attest, check, flag'],
                [<C key="2">TARE_API_URL</C>, <C key="2f">--api-url</C>, <span key="2s">The registry API, <C>{API_URL}</C>.</span>, 'attest, check, flag'],
                [<C key="3">TARE_REGISTRY_PROGRAM_ID</C>, <C key="3f">--registry-program</C>, REGISTRY_PROGRAM_ID ? <span key="3s">The address of the registry program, <C>{REGISTRY_PROGRAM_ID}</C>.</span> : 'The address of the registry program.', 'attest, check, flag'],
                [<C key="4">TARE_KEYPAIR</C>, <C key="4f">--keypair</C>, 'A Solana keypair file. It signs and pays.', 'attest, flag'],
                [<C key="5">TARE_MAINNET_RPC_URL</C>, <C key="5f">--url</C>, <span key="5s">A mainnet RPC endpoint for <C>tare prove --program</C>.</span>, 'prove'],
              ]}
            />
          </>
        ),
      },
      {
        id: 'flags',
        title: 'Flags shared by build, weigh, prove and attest',
        body: (
          <Table
            head={['Flag', 'What it does', 'Default']}
            rows={[
              [<C key="1">--manifest-path &lt;path&gt;</C>, "The program's Cargo.toml. In a workspace of several programs, it selects one.", <C key="d">Cargo.toml</C>],
              [<C key="2">--host</C>, 'Compile and run with the toolchain on this machine instead of the container. Build and weigh or prove in the same mode.', 'Off'],
              [<C key="3">--features &lt;a,b&gt;</C>, "Cargo features of the program, comma-separated. Weigh and prove take the same list the build had.", 'None'],
              [<C key="4">--idl &lt;path&gt;</C>, "An IDL file to use instead of the one generated from the stock build.", 'None'],
              [<C key="5">--tests-corpus &lt;path&gt;</C>, "Adds recorded transactions from the program's own test suite to the corpus.", 'None'],
              [<C key="6">--mainnet-corpus &lt;path&gt;</C>, <span key="6s">Adds a recorded mainnet corpus, as <C>tare prove --program</C> writes it to <C>target/tare/mainnet-corpus.json</C>.</span>, 'None'],
              [<C key="7">--replay &lt;path&gt;</C>, <span key="7s">Runs a replay bundle instead of generating the corpus: the inputs a proof report carries, as <C>tare check</C> uses them.</span>, 'None'],
            ]}
          />
        ),
      },
      {
        id: 'verbose',
        title: 'Seeing every step',
        body: (
          <p>
            <C>--verbose</C>, on any command, prints each command Tare runs (Cargo, <C>cargo-build-sbf</C>, Docker, git) and its full
            output.
          </p>
        ),
      },
    ],
  },

  /* ───────────── Commands: build ───────────── */

  build: {
    lead: 'tare build compiles your workspace twice, once with stock Anchor and once with Tare, and leaves both programs side by side.',
    banner: [{ t: 'tare build', c: 'green' }],
    sections: [
      {
        id: 'synopsis',
        title: 'Synopsis',
        body: (
          <>
            <Shell>{`$ tare build [--manifest-path <path>] [--host]`}</Shell>
            <p>
              Run it where you run <C>anchor build</C>. The flags are in <L href="/docs/install/environment">Environment</L>.
            </p>
          </>
        ),
      },
      {
        id: 'does',
        title: 'What it does',
        body: (
          <Steps>
            <li>Resolves the workspace&rsquo;s dependency graph as stock Anchor would, and records it.</li>
            <li>
              Compiles the program with stock Anchor: the reference, <C>target/deploy/&lt;program&gt;.so</C>, and its IDL.
            </li>
            <li>
              Compiles the same source again with Tare&rsquo;s implementation of the Anchor crates{' '}
              <L href="/docs/build/substitution">substituted at the dependency level</L>.
            </li>
            <li>
              Runs the <L href="/docs/bytecode">bytecode stage</L> over the linked Tare program and writes{' '}
              <C>target/tare/&lt;program&gt;.so</C>.
            </li>
            <li>Restores your Cargo.lock to the bytes it had, or to its absence.</li>
          </Steps>
        ),
      },
      {
        id: 'prints',
        title: 'What it prints',
        body: (
          <>
            <p>Two summary lines end the build:</p>
            <Shell>{`
bytecode: <n> instructions rewritten; <linked> -> <optimized> bytes
<program>: stock <stock> bytes -> tare <tare> bytes
`}</Shell>
            <p>
              Both programs use the same platform tools, the same architecture and the same profile, so the difference between the two
              sizes is the framework.
            </p>
          </>
        ),
      },
      {
        id: 'stops',
        title: 'When the build stops',
        body: (
          <p>
            A construct Tare does not implement stops the build with a diagnostic naming the construct and its location. A program builds
            whole with Tare or it does not build; no part of it falls back to the stock runtime.{' '}
            <L href="/docs/compatibility">Compatibility</L>
          </p>
        ),
      },
    ],
  },

  'build/outputs': {
    lead: 'Everything tare build, weigh and prove leave under target/.',
    banner: [{ t: 'target/tare/', c: 'white' }],
    sections: [
      {
        id: 'programs',
        title: 'Programs',
        body: (
          <Table
            head={['File', 'What it is']}
            rows={[
              [<C key="1">target/deploy/&lt;program&gt;.so</C>, 'The stock build. The reference in every comparison.'],
              [<C key="2">target/idl/&lt;program&gt;.json</C>, "The IDL, from Anchor's own IDL build."],
              [<C key="3">target/tare/&lt;program&gt;.so</C>, 'The Tare build, after the bytecode stage. The one you deploy.'],
              [<C key="4">target/tare/&lt;program&gt;.linked.so</C>, 'The Tare build as linked, before the bytecode stage.'],
              [<C key="5">target/tare/&lt;program&gt;.symbols.so</C>, 'The Tare build with its symbols, for disassembly and debugging.'],
              [<C key="6">target/tare/&lt;program&gt;.stock-symbols.so</C>, 'The stock build with its symbols.'],
            ]}
          />
        ),
      },
      {
        id: 'documents',
        title: 'Documents',
        body: (
          <Table
            head={['File', 'Written by', 'What it holds']}
            rows={[
              [<C key="1">build.json</C>, 'build', 'Both artifacts, their hashes, the Anchor version, the source, the platform tools.'],
              [<C key="2">dependency-graph.json</C>, 'build', 'The resolved graph the Tare build compiled against.'],
              [<C key="2b">stock-dependency-graph.json</C>, 'build', 'The resolved graph the stock build compiled against.'],
              [<C key="2c">adapters/</C>, 'build', "Tare's packages, named and versioned as the Anchor packages they stand in for."],
              [<C key="3">optimization.json</C>, 'build', "The bytecode stage's counters and the hashes before and after."],
              [<C key="4">corpus.json</C>, 'weigh, prove', 'The generated inputs both builds ran.'],
              [<C key="5">execution.json</C>, 'weigh, prove', 'Every transaction outcome on both builds.'],
              [<C key="5b">replay.json</C>, 'weigh, prove', 'Every input of the run with the programs it called: what a check replays.'],
              [<C key="6">weigh.json</C>, 'weigh', <L key="l" href="/docs/weigh/file">The weighing.</L>],
              [<C key="7">proof.json</C>, 'prove', <L key="l" href="/docs/prove/report">The proof report.</L>],
              [<C key="8">mainnet-corpus.json</C>, <span key="8w">prove <C>--program</C></span>, 'The transactions sampled from mainnet, with the account state they replay over.'],
              [<C key="9">check/&lt;record&gt;/</C>, 'check, flag', "The record's source, fetched at its commit and rebuilt."],
            ]}
          />
        ),
      },
      {
        id: 'several',
        title: 'Several programs in one workspace',
        body: (
          <p>
            Each program also gets its own copies, prefixed with its name: <C>&lt;program&gt;.build.json</C>,{' '}
            <C>&lt;program&gt;.weigh.json</C>, <C>&lt;program&gt;.proof.json</C> and so on. The unprefixed files hold the program selected
            last.
          </p>
        ),
      },
      {
        id: 'locks',
        title: 'Lockfiles',
        body: (
          <p>
            The lockfile each compilation resolved is kept as <C>&lt;program&gt;.stock.Cargo.lock</C> and{' '}
            <C>&lt;program&gt;.tare.Cargo.lock</C>, with their hashes in <C>&lt;program&gt;.dependency-locks.json</C>. Your own
            Cargo.lock is left as it was.
          </p>
        ),
      },
    ],
  },

  'build/substitution': {
    lead: 'Tare takes the place of the Anchor crates in the dependency graph, so the source it compiles is yours, untouched.',
    banner: [
      { t: 'anchor-lang', c: 'line' },
      { t: 'tare-lang', c: 'green' },
    ],
    sections: [
      {
        id: 'how',
        title: 'How the substitution is made',
        body: (
          <>
            <p>
              Tare reads the workspace&rsquo;s resolved package graph and presents its own implementation as packages named and
              versioned exactly like the Anchor packages you depend on. Cargo is given a configuration file on the command line with a
              source patch for each one:
            </p>
            <Source>{`
[patch.crates-io]             # Anchor from the registry
[patch."<git URL>"]           # Anchor from a git dependency
[patch."file:///<path>"]      # Anchor from a path dependency
`}</Source>
            <p>Your manifests are not edited and your source is not copied, patched or rewritten. It is compiled in place.</p>
          </>
        ),
      },
      {
        id: 'verified',
        title: 'Verified before compiling',
        body: (
          <p>
            After patching, a second metadata query checks every <C>anchor-*</C> package in the resolved graph. Each one must point at
            Tare&rsquo;s implementation; a single stock framework package left in the graph fails the build. The resolved graph is saved
            with the artifact.
          </p>
        ),
      },
      {
        id: 'same',
        title: 'What stays the same between the two builds',
        body: (
          <List>
            <li>The platform tools and the SBF architecture.</li>
            <li>The build profile and the workspace&rsquo;s settings.</li>
            <li>Your source, your features, your other dependencies.</li>
          </List>
        ),
      },
      {
        id: 'refused',
        title: 'Graphs that are refused',
        body: (
          <p>
            An Anchor version outside the <L href="/docs/compatibility/versions">pinned lines</L>, or two versions of one Anchor package
            in a single graph, stops the build by name.
          </p>
        ),
      },
    ],
  },

  'build/language': {
    lead: 'tare-lang is a second implementation of the Anchor language: the same macros and types, generating leaner code around your handlers.',
    banner: [{ t: 'tare-lang', c: 'green' }],
    sections: [
      {
        id: 'entrypoint',
        title: 'Entrypoint',
        body: (
          <p>
            The entrypoint borrows the loader&rsquo;s input buffer. A cursor walks the account records to find the instruction data and
            the program ID, and materializes an account only when a handler uses it. No vector of account infos is allocated and no
            reference-counted graph is built. Duplicate accounts resolve to the same backing record.
          </p>
        ),
      },
      {
        id: 'dispatch',
        title: 'Dispatch',
        body: (
          <p>
            <C>#[program]</C> emits direct dispatch on the 8-byte discriminator to a wrapper per handler. Instruction-name logs are
            constant byte strings. The legacy IDL instructions keep their own dispatcher, apart from ordinary instruction bodies.
          </p>
        ),
      },
      {
        id: 'accounts',
        title: 'Accounts and constraints',
        body: (
          <>
            <p>
              <C>#[derive(Accounts)]</C> follows stock extraction and constraint order. It covers typed and unchecked accounts, signers,
              programs, interfaces, sysvars, account loaders, optional and boxed accounts and composite groups, with constraints for
              initialization, payer and signature, seeds and bumps, ownership, addresses, relations, custom predicates, zero, close and
              reallocation.
            </p>
            <p>
              Fixed primitive layouts are read and written as views over the account&rsquo;s own bytes, with field offsets verified
              against the Borsh layout. Other layouts use snapshots that are serialized back only when a handler changed them.
            </p>
            <p>No validation check is dropped. What is removed is the work around the checks.</p>
          </>
        ),
      },
      {
        id: 'cpi',
        title: 'CPI and SPL',
        body: (
          <p>
            <C>CpiContext</C>, generated <C>cpi::</C> clients, <C>declare_program!</C> bindings, return values, events and event CPI use
            the same instruction bytes and account order as stock. The <C>anchor-spl</C> surface is implemented for token, token-2022,
            token-interface, associated-token, token extensions, metadata and memo, calling the real SPL programs.
          </p>
        ),
      },
      {
        id: 'errors',
        title: 'Errors',
        body: (
          <p>
            Error codes, messages, account names and compared values stay observable, formatted on the stack. Error order under several
            simultaneous faults is part of what <C>tare prove</C> compares.
          </p>
        ),
      },
    ],
  },

  'build/container': {
    lead: 'Both compilations run in one pinned container image, so the same source at the same commit gives the same two binaries anywhere.',
    banner: [{ t: 'TARE_IMAGE', c: 'white' }],
    sections: [
      {
        id: 'default',
        title: 'The default',
        body: (
          <p>
            Without <C>--host</C>, the CLI starts the image named by <C>TARE_IMAGE</C>, mounts the repository, and runs the same command
            inside it. The image pins its Rust base by digest, the Agave release and platform tools by checksum, and its system packages
            by a dated snapshot.
          </p>
        ),
      },
      {
        id: 'pin',
        title: 'Pinning an image',
        body: (
          <>
            <Shell>{`$ TARE_IMAGE=<repository>@sha256:<digest> tare build`}</Shell>
            <p>
              A digest names one image for good. The toolchain version in a record tells <C>tare check</C> which one to rebuild with.
            </p>
          </>
        ),
      },
      {
        id: 'host',
        title: 'Building on the host',
        body: (
          <>
            <Shell>{`$ tare build --host`}</Shell>
            <p>Uses Cargo and the platform tools installed on your machine. It is the quick loop while you work.</p>
          </>
        ),
      },
    ],
  },

  'build/deploy': {
    lead: 'The Tare build goes on chain the way any upgrade does, with the Solana CLI, at the address your program already has.',
    banner: [{ t: 'solana program deploy', c: 'white' }],
    sections: [
      {
        id: 'deploy',
        title: 'Deploy',
        body: (
          <>
            <Shell>{`$ solana program deploy target/tare/<program>.so --program-id <PROGRAM_ID>`}</Shell>
            <p>The program at the same address then runs the Tare build over its existing accounts.</p>
          </>
        ),
      },
      {
        id: 'before',
        title: 'Before you deploy',
        body: (
          <Steps>
            <li>
              <C>tare prove</C> exits zero on the build you are about to deploy. <L href="/docs/prove">Prove</L>
            </li>
            <li>
              Add your program&rsquo;s own mainnet transactions to the corpus with <C>--program &lt;PROGRAM_ID&gt;</C>.
            </li>
            <li>
              <C>tare attest</C>, so your users and integrators can check the build themselves. <L href="/docs/check/attest">tare attest</L>
            </li>
          </Steps>
        ),
      },
      {
        id: 'smaller',
        title: 'A smaller binary',
        body: (
          <p>
            Rent is charged per byte of program data. <C>tare weigh</C> reports the rent each binary locks, so you can see what the
            smaller program frees. <L href="/docs/weigh/reading">Reading a weighing</L>
          </p>
        ),
      },
    ],
  },

  /* ───────────── Commands: weigh ───────────── */

  weigh: {
    lead: 'tare weigh runs every instruction on both builds and prints gross, tare and net for each.',
    banner: [
      { t: 'tare weigh', c: 'magenta' },
    ],
    sections: [
      {
        id: 'synopsis',
        title: 'Synopsis',
        body: (
          <>
            <Shell>{`$ tare weigh [--manifest-path <path>] [--host] [--features <a,b>] [--tests-corpus <path>]`}</Shell>
            <p>
              Every flag is in <L href="/docs/install/environment">Environment</L>.
            </p>
          </>
        ),
      },
      {
        id: 'output',
        title: 'What it prints',
        body: (
          <>
            <Shell>{WEIGH_OUTPUT ? `$ tare weigh${NL}${WEIGH_OUTPUT}${PROOF_LINE ? NL + PROOF_LINE : ''}` : `$ tare weigh`}</Shell>
            {FROM_BENCH}
            <p>
              One row per instruction in the IDL, then the total, then the result of comparing the two builds over the same inputs.{' '}
              <L href="/docs/weigh/reading">Reading a weighing</L>
            </p>
          </>
        ),
      },
      {
        id: 'does',
        title: 'What it does',
        body: (
          <Steps>
            <li>
              Reads the two programs <C>tare build</C> left, and checks they are unchanged since. Without a build it stops with{' '}
              <C>run tare build --host first</C>; after a rebuild with another Tare version, with another feature list or in the other
              mode, it stops and says which.
            </li>
            <li>Generates inputs for every instruction from the IDL, with deterministic signers and accounts.</li>
            <li>Runs each input on the stock build and on the Tare build in a local SVM, at the same address, over the same state.</li>
            <li>
              Writes the weighing to <C>target/tare/weigh.json</C>. <L href="/docs/weigh/file">weigh.json</L>
            </li>
          </Steps>
        ),
      },
    ],
  },

  'weigh/reading': {
    lead: 'What each column of a weighing means, and how the figures behind it are taken.',
    banner: [
      { t: 'gross', c: 'line' },
      { t: 'tare', c: 'magenta' },
      { t: 'net', c: 'green' },
    ],
    sections: [
      {
        id: 'columns',
        title: 'Columns',
        body: (
          <Table
            head={['Column', 'What it is']}
            rows={[
              ['instruction', 'Its name from the IDL.'],
              ['gross', 'Median compute units of a successful call on the stock build.'],
              ['tare', 'Gross minus net.'],
              ['net', 'Median compute units of the same calls on the Tare build.'],
              ['inputs', 'How many successful calls the medians were taken over.'],
            ]}
          />
        ),
      },
      {
        id: 'median',
        title: 'Medians, not estimates',
        body: (
          <p>
            A figure is the upper middle value of what the meter reported, so it is always a number of units some call really burned. An
            instruction no input reached prints <C>not measured (0 successful inputs)</C> and adds nothing to the totals.
          </p>
        ),
      },
      {
        id: 'total',
        title: 'The total',
        body: <p>The sum of the measured instructions&rsquo; medians: one call of each. Its inputs column is the count of all calls.</p>,
      },
      {
        id: 'size',
        title: 'Binary size and rent',
        body: (
          <p>
            The weighing also holds each build&rsquo;s size in bytes and the lamports it locks as rent, computed from the same rent
            configuration for both.
          </p>
        ),
      },
    ],
  },

  'weigh/file': {
    lead: 'target/tare/weigh.json is the weighing as a document. Its schema is published with the API contract.',
    banner: [{ t: 'weigh.json', c: 'white' }],
    sections: [
      {
        id: 'document',
        title: 'The document',
        body: (
          <>
            {WEIGH_DOCUMENT ? <Source>{WEIGH_DOCUMENT}</Source> : <p>The first benchmark run fills this in.</p>}
            {WEIGH_DOCUMENT && featured && (
              <p>
                {featured.name} from <L href="/bench">the benchmark</L>, with its instruction list cut to two.
              </p>
            )}
          </>
        ),
      },
      {
        id: 'fields',
        title: 'Fields',
        body: (
          <Table
            head={['Field', 'What it holds']}
            rows={[
              [<C key="1">schemaVersion</C>, 'The version of this document format.'],
              [<C key="2">program</C>, "The program's name and, when it is known, its address."],
              [<C key="3">toolchainVersion</C>, 'What tare --version prints.'],
              [<C key="4">anchorVersion</C>, 'The Anchor version the workspace pins.'],
              [<C key="4b">buildEnvironment</C>, 'The image or host the two builds ran in. tare check refuses a rebuild in a different one.'],
              [<C key="4c">accountAbi</C>, 'How the Tare build reads its accounts.'],
              [<C key="5">source</C>, 'The repository and the commit that was built.'],
              [<C key="6">builds.stock, builds.tare</C>, 'Each binary: its SHA-256, its size in bytes, the lamports it locks.'],
              [<C key="7">instructions[]</C>, 'Name, discriminator, gross, net, tare, inputs. Weights are null when inputs is 0.'],
              [<C key="8">totals</C>, 'gross, net, tare and inputs, summed over measured instructions.'],
              [<C key="9">proof</C>, <span key="9s">The comparison of the two builds over the same inputs, which weigh runs too. <L href="/docs/prove/report">proof.json</L></span>],
              [<C key="10">optimization</C>, "The bytecode stage's counters and hashes, as in optimization.json."],
              [<C key="11">recordedCorpus</C>, 'For each recorded corpus that ran: how many of its inputs reproduced the outcome recorded on the stock build.'],
              [<C key="12">replay</C>, 'Every input of the run, gzip-compressed and base64-encoded, with its SHA-256. It is what a check replays.'],
            ]}
          />
        ),
      },
      {
        id: 'numbers',
        title: 'Numbers',
        body: (
          <p>
            Compute units, lamports, byte sizes and counts are JSON integers. Tare is signed, since an instruction can in principle weigh
            more after a build than before it.
          </p>
        ),
      },
    ],
  },

  'weigh/deployed': {
    lead: 'The site weighs any deployed program by its address, with no install and no source.',
    banner: [
      { t: 'weigh', c: 'line' },
      { t: 'a program', c: 'green' },
    ],
    sections: [
      {
        id: 'how',
        title: 'How it reads a program',
        body: (
          <Steps>
            <li>Reads the program account and its data account: the binary&rsquo;s size and the rent it locks.</li>
            <li>Reads the IDL account when the program has one on chain, for the instruction names.</li>
            <li>Takes a bounded sample of the program&rsquo;s recent mainnet transactions.</li>
            <li>Groups their compute by instruction discriminator.</li>
          </Steps>
        ),
      },
      {
        id: 'shows',
        title: 'What the page shows',
        body: (
          <Table
            head={['Column', 'What it is']}
            rows={[
              ['Share', "The instruction's part of all the compute the sample burned in this program."],
              ['Calls', 'How many sampled calls it had.'],
              ['Median', 'Median compute units of a call.'],
              ['p90', 'The 90th percentile.'],
              ['Of a transaction', `The p90 as a share of the ${units(LIMITS.transactionUnits)}-unit cap.`],
            ]}
          />
        ),
      },
      {
        id: 'unnamed',
        title: 'Instructions without a name',
        body: <p>A program with no IDL on chain is still weighed. Its instructions are listed by their discriminator in hex.</p>,
      },
      {
        id: 'exact',
        title: 'From a sample to the three weights',
        body: (
          <p>
            The page shows what the deployed build burns: one number per instruction. Gross, tare and net come from the source, with{' '}
            <C>tare weigh</C>. Go to <L href="/weigh">Weigh</L>.
          </p>
        ),
      },
    ],
  },

  /* ───────────── Commands: prove ───────────── */

  prove: {
    lead: 'tare prove runs the stock build and the Tare build over the same inputs and compares the result of every transaction.',
    banner: [{ t: 'tare prove', c: 'cyan' }],
    sections: [
      {
        id: 'synopsis',
        title: 'Synopsis',
        body: (
          <>
            <Shell>{`$ tare prove [--manifest-path <path>] [--host] [--features <a,b>] [--tests-corpus <path>]
           [--program <address> [--samples <n>] [--url <rpc>]]`}</Shell>
            <Table
              head={['Flag', 'What it adds']}
              rows={[
                [<C key="1">--tests-corpus &lt;path&gt;</C>, "Transactions recorded from the program's own test suite."],
                [<C key="2">--program &lt;address&gt;</C>, "Transactions sampled from mainnet for that address, replayed over current account state. The address must be the one in the program's IDL."],
                [<C key="3">--samples &lt;n&gt;</C>, <span key="3s">How many recent signatures <C>--program</C> reads, from 1 to 20. Default 3.</span>],
                [<C key="4">--url &lt;rpc&gt;</C>, <span key="4s">The mainnet RPC endpoint <C>--program</C> reads through, or <C>TARE_MAINNET_RPC_URL</C>.</span>],
              ]}
            />
            <p>
              The rest are in <L href="/docs/install/environment">Environment</L>.
            </p>
          </>
        ),
      },
      {
        id: 'output',
        title: 'What it prints',
        body: (
          <>
            <Shell>{WEIGH_OUTPUT && PROOF_LINE ? `$ tare prove${NL}${WEIGH_OUTPUT}${NL}${PROOF_LINE}` : `$ tare prove`}</Shell>
            {FROM_BENCH}
            <p>
              The weighing, then one line: <C>matched</C> or <C>diverged</C>, and the number of transactions compared.
            </p>
          </>
        ),
      },
      {
        id: 'exit',
        title: 'Exit code',
        body: (
          <Table
            head={['Code', 'Meaning']}
            rows={[
              [<C key="1">0</C>, 'Every comparison matched.'],
              [<C key="2">1</C>, <span key="s">The two builds differ on an input. <L href="/docs/prove/divergence">A divergence</L></span>],
            ]}
          />
        ),
      },
      {
        id: 'states',
        title: 'What a proof states',
        body: (
          <p>
            That the two builds ended in the same observable state on every input of a named corpus. The corpus is named by its hash and
            its three counts, in the report and in the record, so anyone can rerun exactly that. <L href="/docs/prove/report">proof.json</L>
          </p>
        ),
      },
    ],
  },

  'prove/corpora': {
    lead: 'A corpus is an ordered set of transaction sequences with their starting accounts. A proof draws on three of them.',
    banner: CORPORA.map((c) => ({ t: c.label.toLowerCase(), c: 'cyan' as const })),
    sections: [
      {
        id: 'three',
        title: 'The three',
        body: <Table head={['Corpus', 'Its inputs']} rows={CORPORA.map((c) => [<b key={c.key}>{c.label}</b>, c.says])} />,
      },
      {
        id: 'generated',
        title: 'Generated',
        body: (
          <>
            <p>
              Sequences are generated from the stock IDL with deterministic signer keys, so the same program gives the same corpus.
              Program-derived addresses, fixed addresses, signer funding and the order of initializers are resolved from the IDL and from
              hints read out of your source.
            </p>
            <p>
              Successful transactions feed the calls that follow them in a sequence. Negative variants, inputs built to fail, run without
              committing their changes: an error path is compared like any other result.
            </p>
          </>
        ),
      },
      {
        id: 'tests',
        title: 'Own tests',
        body: (
          <>
            <p>
              Your test suite is recorded once as it runs on the stock build: the signed transactions, the starting and changing account
              state, the slots. The recording replays on both builds.
            </p>
            <Shell>{`$ tare prove --tests-corpus <path/to/tests.json>`}</Shell>
          </>
        ),
      },
      {
        id: 'mainnet',
        title: 'Mainnet',
        body: (
          <>
            <Shell>{`$ TARE_MAINNET_RPC_URL=<rpc> tare prove --program <PROGRAM_ID> --samples 20`}</Shell>
            <p>
              Reads the address&rsquo;s most recent signatures, keeps the transactions that call the program directly or through a CPI,
              and replays each over the accounts as they stand now, with every other program they call loaded from mainnet. It is a
              bounded number of RPC reads, made once when you run the command. The sample is kept in{' '}
              <C>target/tare/mainnet-corpus.json</C>. When none of the sampled transactions call the program, it stops with{' '}
              <C>no replayable transactions found in the bounded sample</C>.
            </p>
          </>
        ),
      },
      {
        id: 'hash',
        title: 'The corpus hash',
        body: (
          <p>
            Every corpus is hashed. The hash and the count from each of the three go into the proof report and the record, which is what
            makes a proof a statement about a specific set of inputs.
          </p>
        ),
      },
    ],
  },

  'prove/comparison': {
    lead: 'After every transaction, the two builds must have left the same observable state.',
    banner: [
      { t: 'stock build', c: 'line' },
      { t: '==', c: 'green' },
      { t: 'tare build', c: 'white' },
    ],
    sections: [
      {
        id: 'compared',
        title: 'What is compared',
        body: (
          <Table
            head={['', 'Compared']}
            rows={[
              ['Account bytes', "Every account's data, byte for byte."],
              ['Lamports', "Every account's balance."],
              ['Ownership and metadata', "Each account's owner, executable flag and rent metadata."],
              ['Logs', 'Every log line, events included.'],
              ['Return data', 'The program that set it and its bytes.'],
              ['Errors', 'The transaction error, exactly as the runtime reports it.'],
            ]}
          />
        ),
      },
      {
        id: 'ignored',
        title: 'The one line that is not compared',
        body: (
          <p>
            The runtime&rsquo;s own <C>Program &lt;address&gt; consumed N of M compute units</C> lines differ by design: that
            difference is the tare. Nothing else in the logs is skipped.
          </p>
        ),
      },
      {
        id: 'setup',
        title: 'How the two runs are set up',
        body: (
          <List>
            <li>Each build is loaded at the same program address.</li>
            <li>Both start from the same accounts, built-ins, SPL programs and sysvars, under the same metering.</li>
            <li>Sibling programs in the workspace use the same stock binary on both sides.</li>
            <li>State is kept across the transactions of a sequence, on each side separately.</li>
          </List>
        ),
      },
      {
        id: 'counts',
        title: 'Comparison counts',
        body: (
          <p>
            The report counts the comparisons actually made: inputs, account bytes, lamports, logs, return data and errors. They are in{' '}
            <L href="/docs/prove/report">proof.json</L>.
          </p>
        ),
      },
    ],
  },

  'prove/divergence': {
    lead: 'One input on which the two builds differ fails the proof.',
    banner: [{ t: 'diverged', c: 'red' }],
    sections: [
      {
        id: 'prints',
        title: 'What the command does',
        body: (
          <Steps>
            <li>
              Runs the whole corpus, then prints <C>proof: diverged</C> with the number of transactions compared.
            </li>
            <li>
              Prints the first divergence as JSON on standard error, and keeps it in <C>target/tare/proof.json</C> under{' '}
              <C>proof.firstDivergence</C>.
            </li>
            <li>Exits with code 1. Nothing is attested from a failed proof.</li>
          </Steps>
        ),
      },
      {
        id: 'holds',
        title: 'What a divergence holds',
        body: (
          <Table
            head={['Field', 'What it holds']}
            rows={[
              [<C key="1">input</C>, 'The transaction bytes, the corpus it came from, its label, the accounts it started from.'],
              [<C key="2">instructionIndex</C>, 'Which instruction of the transaction diverged.'],
              [
                <C key="3">differences[]</C>,
                <span key="3s">
                  Each difference by <C>kind</C>: <C>account_bytes</C>, <C>lamports</C>, <C>account_metadata</C> (owner, executable,
                  rent epoch), <C>logs</C>, <C>return_data</C> or <C>error</C>. <C>gross</C> holds the stock build&rsquo;s value and{' '}
                  <C>net</C> the Tare build&rsquo;s; the account kinds also name the <C>account</C>.
                </span>,
              ],
            ]}
          />
        ),
      },
      {
        id: 'flag',
        title: 'The same input, as a flag',
        body: (
          <p>
            A divergence found against a build that is already attested is exactly what <C>tare flag</C> files against its record.{' '}
            <L href="/docs/check/flag">tare flag</L>
          </p>
        ),
      },
    ],
  },

  'prove/report': {
    lead: 'target/tare/proof.json is the weighing with its proof. It is the document a record pins and commits to.',
    banner: [{ t: 'proof.json', c: 'white' }],
    sections: [
      {
        id: 'document',
        title: 'The proof in the document',
        body: (
          <>
            {PROOF_DOCUMENT ? <Source>{PROOF_DOCUMENT}</Source> : <p>The first benchmark run fills this in.</p>}
            {PROOF_DOCUMENT && featured && (
              <p>
                The proof of {featured.name} from <L href="/bench">the benchmark</L>. The rest of the document is the{' '}
                <L href="/docs/weigh/file">weighing</L>.
              </p>
            )}
          </>
        ),
      },
      {
        id: 'fields',
        title: 'Fields',
        body: (
          <Table
            head={['Field', 'What it holds']}
            rows={[
              [<C key="1">status</C>, 'matched or diverged.'],
              [<C key="2">corpus.hash</C>, 'SHA-256 of the corpus the builds ran.'],
              [<C key="3">corpus.counts</C>, 'Inputs from each of the three corpora: tests, generated, mainnet.'],
              [<C key="4">comparisons</C>, 'Inputs compared, and the count of each kind of comparison made.'],
              [<C key="5">firstDivergence</C>, 'Null when matched; the diverging input and its differences otherwise.'],
            ]}
          />
        ),
      },
      {
        id: 'pinned',
        title: 'Pinned by its hash',
        body: (
          <p>
            <C>tare attest</C> uploads this document and writes its SHA-256 to the record. The registry page reads it back by that hash
            to show the weights by instruction.
          </p>
        ),
      },
    ],
  },

  /* ───────────── Commands: attest and check ───────────── */

  check: {
    lead: 'A proof on your machine becomes a record on chain, which anyone can rerun and anyone can dispute.',
    banner: [
      { t: 'attest', c: 'blue' },
      { t: 'check', c: 'white' },
      { t: 'flag', c: 'red' },
    ],
    sections: [
      {
        id: 'three',
        title: 'Three commands, three people',
        body: (
          <Table
            head={['Command', 'Who runs it', 'What it leaves']}
            rows={[
              [<L key="1" href="/docs/check/attest">tare attest</L>, "The program's team", 'A record on the registry.'],
              [<L key="2" href="/docs/check/check">tare check</L>, 'Anyone', 'matches, or the first difference.'],
              [<L key="3" href="/docs/check/flag">tare flag</L>, 'Anyone with a diverging input', 'A flag on the record.'],
            ]}
          />
        ),
      },
      {
        id: 'record',
        title: 'The record',
        body: (
          <p>
            A record is an account of the <L href="/docs/registry">registry program</L> holding the two build hashes, the source commit,
            the corpus and the weights. Each has a page in <L href="/builds">the registry</L>.{' '}
            <L href="/docs/check/record">Reading a record</L>
          </p>
        ),
      },
      {
        id: 'cost',
        title: 'What it costs',
        body: (
          <p>
            Attesting carries a fee in SOL, paid by the attester to the token&rsquo;s launch wallet. Checking costs nothing: it runs on
            your machine. <L href="/docs/token">Token</L>
          </p>
        ),
      },
    ],
  },

  'check/attest': {
    lead: 'tare attest publishes the proof in target/tare/proof.json to the registry: the report is pinned and a record is written on chain.',
    banner: [{ t: 'tare attest', c: 'blue' }],
    sections: [
      {
        id: 'synopsis',
        title: 'Synopsis',
        body: (
          <>
            <Shell>{`$ tare attest [--manifest-path <path>] [--keypair <path>] [--url <rpc>] [--api-url <url>] [--registry-program <address>]`}</Shell>
            <p>
              Each of the four network flags can be set once as a variable instead. <L href="/docs/install/environment#network">Variables for the registry</L>
            </p>
          </>
        ),
      },
      {
        id: 'before',
        title: 'Before you attest',
        body: (
          <Steps>
            <li>
              The workspace is a git repository whose <C>origin</C> is a public URL, with no credentials or query string in it. The
              record names that repository, and <C>tare check</C> fetches from it.
            </li>
            <li>
              The commit you built is pushed, and nothing in the tree is modified or untracked: <C>git status</C> is empty.
            </li>
            <li>
              <C>tare build</C>, then <C>tare prove</C>, at that commit, and the proof matched. Attest reads{' '}
              <C>target/tare/&lt;program&gt;.proof.json</C> and both binaries as they were built.
            </li>
            <li>A keypair holding SOL for the attest fee, the record&rsquo;s rent and the transaction fee.</li>
          </Steps>
        ),
      },
      {
        id: 'run',
        title: 'Run it',
        body: (
          <>
            <Shell>{`$ export TARE_RPC_URL=<rpc>
$ export TARE_API_URL=${API_URL}
$ export TARE_REGISTRY_PROGRAM_ID=${REGISTRY_PROGRAM_ID || '<registry program>'}
$ export TARE_KEYPAIR=~/.config/solana/id.json
$ tare attest`}</Shell>
            <Shell>{ATTEST_CAPTURE}</Shell>
            <p>
              Squads v4, captured on a local validator. The first line is the registry&rsquo;s attest fee as its <C>Config</C> holds it
              at that moment, printed before anything is sent. The last line is the record&rsquo;s page.
            </p>
          </>
        ),
      },
      {
        id: 'does',
        title: 'What it does',
        body: (
          <Steps>
            <li>Checks the proof matched, the tree is clean, and the proof&rsquo;s repository and commit are the origin and HEAD.</li>
            <li>Reads the attest fee from the registry&rsquo;s config and prints it.</li>
            <li>Checks the SHA-256 of both binaries against the proof.</li>
            <li>
              Uploads the report to <C>POST /v1/reports</C>: the proof document without its <C>optimization</C> and{' '}
              <C>recordedCorpus</C> fields. The API pins it and returns its SHA-256 and location; the CLI checks the hash against the bytes
              it sent.
            </li>
            <li>
              Sends one <C>attest</C> transaction signed by your keypair. It creates the record at the address derived from the program,
              the report hash and your key, and pays the fee from your key to the fee recipient the registry&rsquo;s config names.
            </li>
            <li>Prints the record, the transaction signature and the page URL.</li>
          </Steps>
        ),
      },
      {
        id: 'once',
        title: 'One record per report and attester',
        body: (
          <p>
            The record&rsquo;s address is derived from <C>record</C>, the program, the report hash and the attester, so the same key cannot
            attest the same report twice. A new commit, a new corpus or a new toolchain gives a new report and a new record. Records are
            never changed or closed.
          </p>
        ),
      },
      {
        id: 'refusals',
        title: 'When it refuses',
        body: (
          <Table
            head={['It prints', 'What to do']}
            rows={[
              [<C key="1">attestation requires a matching differential proof</C>, <span key="1s">The last <C>tare prove</C> diverged. <L href="/docs/prove/divergence">A divergence</L></span>],
              [<C key="2">refusing to attest a dirty source tree</C>, 'Commit or remove every change, including untracked files, then build and prove again.'],
              [<C key="3">workspace must be in git</C>, 'Attest from a git repository.'],
              [<C key="4">workspace needs a source origin</C>, <span key="4s">Add the public repository as <C>origin</C>.</span>],
              [<C key="5">attestation needs a public source URL</C>, <span key="5s"><C>origin</C> has to parse as a URL.</span>],
              [<C key="6">refusing to publish credentials or query parameters in source URL</C>, <span key="6s">Remove the user, password or query from <C>origin</C>.</span>],
              [<C key="7">proof source does not match the current git origin and commit</C>, 'You committed or switched branches after proving. Build and prove at HEAD.'],
              [<C key="8">proof stock build hash does not match the current artifact</C>, <span key="8s">A binary changed since the proof (<C>tare</C> for the Tare build). Build and prove again.</span>],
              [<C key="9">report exceeds the API&rsquo;s 1 MiB limit</C>, 'The report with its replay inputs is larger than the API takes.'],
              [<C key="10">supply --url or TARE_RPC_URL</C>, <span key="10s">A network setting is missing; the message names it. The same for the keypair, the API and the registry program.</span>],
              [<C key="11">transaction failed: …</C>, "The registry refused the transaction; the runtime's error follows. A record already at that address is one cause."],
            ]}
          />
        ),
      },
      {
        id: 'next',
        title: 'After',
        body: (
          <p>
            The record is on chain and its page lists it in <L href="/builds">the registry</L> once indexed. Anyone can now rerun it.{' '}
            <L href="/docs/check/check">tare check</L>
          </p>
        ),
      },
    ],
  },

  'check/record': {
    lead: 'What a record holds, and where each part of it is shown.',
    banner: [{ t: 'record', c: 'blue' }],
    sections: [
      {
        id: 'holds',
        title: 'On chain',
        body: (
          <Table
            head={['Field', 'What it is']}
            rows={[
              ['Program', 'The address the build is for.'],
              ['Stock build hash, Tare build hash', 'SHA-256 of each binary.'],
              ['Source', 'The repository and the commit.'],
              ['Toolchain version', 'Which tare made the build.'],
              ['Corpus', 'Its hash and its three counts.'],
              ['Gross and net totals', 'The summed weights.'],
              ['Report', 'The pinned proof report: its hash and where it is.'],
              ['Attester, slot', 'Who wrote the record, and when.'],
            ]}
          />
        ),
      },
      {
        id: 'page',
        title: 'Its page',
        body: (
          <p>
            <C>/builds/&lt;record&gt;</C> shows all of that, with the weights by instruction read from the pinned report, the corpus in
            its three kinds, the check line with a copy button, and every flag filed against it with its input.
          </p>
        ),
      },
      {
        id: 'find',
        title: 'Finding records',
        body: (
          <p>
            <L href="/builds">The registry</L> lists every record, newest first, and filters by program address. A program&rsquo;s own
            records also appear on its <L href="/weigh">weigh</L> page.
          </p>
        ),
      },
    ],
  },

  'check/check': {
    lead: 'tare check rebuilds a record from the record alone and tells you whether it holds. It needs no key and nothing from the attester.',
    banner: [{ t: 'tare check', c: 'white' }],
    sections: [
      {
        id: 'synopsis',
        title: 'Synopsis',
        body: (
          <>
            <Shell>{`$ tare check <RECORD> [--host] [--directory <path>] [--url <rpc>] [--api-url <url>] [--registry-program <address>]`}</Shell>
            <Table
              head={['Flag', 'What it does', 'Default']}
              rows={[
                [<C key="1">--host</C>, "Builds with the toolchain on this machine. Its tare has to be the record's toolchain version.", 'Off: the pinned container'],
                [<C key="2">--directory &lt;path&gt;</C>, 'Where the source is fetched and rebuilt.', <C key="2d">target/tare/check/&lt;record&gt;</C>],
              ]}
            />
            <p>
              The network flags, or their variables, are in <L href="/docs/install/environment#network">Variables for the registry</L>.
            </p>
          </>
        ),
      },
      {
        id: 'needs',
        title: 'What it needs',
        body: (
          <List>
            <li>
              <C>git</C>, and Docker unless you pass <C>--host</C>.
            </li>
            <li>An RPC endpoint on the registry&rsquo;s cluster, the registry API and the registry program&rsquo;s address.</li>
            <li>Any folder. It does not have to be inside a workspace.</li>
          </List>
        ),
      },
      {
        id: 'run',
        title: 'Run it',
        body: (
          <>
            <Shell>{CHECK_CAPTURE}</Shell>
            <p>
              Squads v4, captured on a local validator. Each line is a step below; the last is the verdict.
            </p>
          </>
        ),
      },
      {
        id: 'does',
        title: 'What it does',
        body: (
          <Steps>
            <li>Reads the record from the registry.</li>
            <li>
              Fetches the report from <C>GET /v1/reports/&lt;hash&gt;</C> and checks its SHA-256 is the record&rsquo;s report hash. Then
              checks the report states the same program, build hashes, source, toolchain version, corpus hash, counts and totals as the
              record, and a matched proof.
            </li>
            <li>
              Checks the build environment: the image the record was built in, or the host. A different one stops the check and names
              both.
            </li>
            <li>
              Fetches the source at the record&rsquo;s commit into the directory, with its submodules, and refuses the directory if it is
              at another commit or modified.
            </li>
            <li>Builds both programs with the record&rsquo;s features and IDL, and compares the two hashes with the record.</li>
            <li>Reruns every input of the proof, from the replay inputs the report carries, and compares the corpus hash.</li>
            <li>
              Prints <C>matches</C> and exits 0.
            </li>
          </Steps>
        ),
      },
      {
        id: 'differs',
        title: 'When it does not match',
        body: (
          <>
            <p>
              It stops at the first difference, prints it after <C>error:</C> and exits 1. The values it can name:
            </p>
            <Table
              head={['It prints', 'Meaning']}
              rows={[
                [<C key="1">stock build hash differs: expected &lt;hash&gt;, got &lt;hash&gt;</C>, 'The source at that commit does not build into the stock binary the record names.'],
                [<C key="2">tare build hash differs: expected &lt;hash&gt;, got &lt;hash&gt;</C>, 'The same for the Tare binary.'],
                [<C key="3">corpus hash differs: expected &lt;hash&gt;, got &lt;hash&gt;</C>, 'The rerun did not use the same inputs.'],
                [<C key="4">record/report &lt;field&gt; differs</C>, "The pinned report does not say what the record says."],
                [<C key="5">report content hash mismatch</C>, 'The API served bytes that are not the pinned report.'],
                [<C key="6">build environment differs: record &lt;a&gt;, current &lt;b&gt;</C>, <span key="6s">Rebuild in the recorded environment: the image named, through <C>TARE_IMAGE</C>, or the host.</span>],
                [<C key="7">record requires Tare &lt;version&gt;; this host has &lt;version&gt;</C>, <span key="7s">With <C>--host</C>, install that version (<C>TARE_VERSION</C>), or drop <C>--host</C>.</span>],
                [<C key="8">refusing a dirty check directory</C>, 'Something changed in the directory since it was fetched. Remove it and run again.'],
              ]}
            />
            <p>
              A proof that diverges on the rerun exits 1 with the divergence, as <L href="/docs/prove/divergence">tare prove</L> does.
            </p>
          </>
        ),
      },
      {
        id: 'page',
        title: 'From a record page',
        body: (
          <p>
            Every record&rsquo;s page in <L href="/builds">the registry</L> carries its check line with a copy button.
          </p>
        ),
      },
    ],
  },

  'check/flag': {
    lead: 'tare flag files, against a record, an input on which its two builds differ. The divergence has to reproduce on your machine first.',
    banner: [{ t: 'tare flag', c: 'red' }],
    sections: [
      {
        id: 'synopsis',
        title: 'Synopsis',
        body: (
          <>
            <Shell>{`$ tare flag <RECORD> --input <file> [--host] [--directory <path>] [--keypair <path>] [--url <rpc>] [--api-url <url>] [--registry-program <address>]`}</Shell>
            <p>
              <C>--host</C> and <C>--directory</C> work as in <L href="/docs/check/check">tare check</L>. The network flags and the
              keypair are in <L href="/docs/install/environment#network">Variables for the registry</L>. The keypair pays the
              flag&rsquo;s rent and the transaction fee.
            </p>
          </>
        ),
      },
      {
        id: 'input',
        title: 'The input file',
        body: (
          <>
            <p>
              One transaction and the accounts it starts from, as JSON. A failed proof already holds one: <C>tare prove</C> writes its
              first divergence to <C>target/tare/proof.json</C>, and its <C>input</C> is the file.
            </p>
            <Shell>{`$ jq '.proof.firstDivergence.input' target/tare/proof.json > input.json`}</Shell>
            <Table
              head={['Field', 'What it holds']}
              rows={[
                [<C key="1">transactionBase64</C>, 'The signed transaction, serialized and base64-encoded.'],
                [<C key="2">accounts[]</C>, <span key="2s">Every account it starts from: <C>address</C>, <C>owner</C>, <C>lamports</C>, <C>executable</C>, <C>dataBase64</C>, and <C>rentEpoch</C> (0 when left out).</span>],
                [<C key="3">label</C>, 'Optional. A name for the input.'],
                [<C key="4">slot</C>, 'Optional. The slot it runs at.'],
              ]}
            />
            <p>
              The file may also be a whole flag document, with the input under <C>divergence.input</C>, as the API serves one back.
            </p>
          </>
        ),
      },
      {
        id: 'does',
        title: 'What it does',
        body: (
          <Steps>
            <li>
              Rebuilds the record exactly as <L href="/docs/check/check">tare check</L> does, and stops at the same differences.
            </li>
            <li>Runs the input on the rebuilt stock program and the rebuilt Tare program.</li>
            <li>
              Uploads <C>{`{ schemaVersion, record, divergence }`}</C> to <C>POST /v1/flag-inputs</C>, which pins it and returns its
              SHA-256 and location.
            </li>
            <li>
              Sends one <C>flag</C> transaction that writes a flag with the record, the input&rsquo;s hash and its location, then prints
              the flag&rsquo;s address and the signature.
            </li>
          </Steps>
        ),
      },
      {
        id: 'output',
        title: 'What it prints',
        body: (
          <>
            <p>
              The rebuild&rsquo;s lines, as in a check, then <C>flag: &lt;address&gt;</C> and <C>transaction: &lt;signature&gt;</C>. An
              input on which the two builds agree is refused before anything is uploaded or sent:
            </p>
            <Shell>{FLAG_REFUSED_CAPTURE}</Shell>
            <p>Captured on a local validator.</p>
          </>
        ),
      },
      {
        id: 'once',
        title: 'One flag per input',
        body: (
          <p>
            The flag&rsquo;s address is derived from <C>flag</C>, the record and the input&rsquo;s hash, so one input is filed against a
            record once, by whoever files it first.
          </p>
        ),
      },
      {
        id: 'shown',
        title: 'Where it shows',
        body: <p>On the record&rsquo;s page and in the registry list, with the input and which of the compared things differed.</p>,
      },
    ],
  },

  /* ───────────── Reference ───────────── */

  compatibility: {
    lead: 'The constructs of the Anchor language that tare-lang implements, generated from the benchmark runs.',
    banner: [{ t: 'compatibility', c: 'line' }],
    sections: [
      { id: 'matrix', title: 'Constructs', body: <Compatibility /> },
      {
        id: 'reading',
        title: 'Reading the matrix',
        body: (
          <p>
            A construct is marked for a version once a corpus program using it has built with Tare and its proof matched. The last column
            names those programs; each is in <L href="/bench">the benchmark</L>.
          </p>
        ),
      },
      {
        id: 'outside',
        title: 'A construct outside the matrix',
        body: (
          <p>
            <C>tare build</C> stops and names the construct and where it is used. Your stock build is unaffected.
          </p>
        ),
      },
    ],
  },

  'compatibility/versions': {
    lead: 'Tare builds against four pinned Anchor versions.',
    banner: [
      { t: '0.31.1', c: 'line' },
      { t: '0.32.1', c: 'line' },
      { t: '0.32.3', c: 'line' },
      { t: '1.2.2', c: 'line' },
    ],
    sections: [
      {
        id: 'lines',
        title: 'The versions',
        body: (
          <>
            <Table
              head={['Anchor', 'Notes']}
              rows={[
                ['0.31.1', 'Built-in IDL instructions, as stock.'],
                ['0.32.1', 'Built-in IDL instructions, as stock.'],
                ['0.32.3', 'Built-in IDL instructions, as stock.'],
                ['1.2.2', 'No legacy IDL management by default; #[program(legacy_idl)] opts into the legacy dispatcher.'],
              ]}
            />
            <p>
              Any other <C>anchor-lang</C> version stops the build before it compiles, naming the version and the manifest line that
              asks for it:
            </p>
            <Shell>{`error: unsupported Anchor version <version> at <path>/Cargo.toml:<line>; supported: 0.31.1, 0.32.1, 0.32.3, 1.2.2`}</Shell>
          </>
        ),
      },
      {
        id: 'selected',
        title: 'How a version is selected',
        body: (
          <p>
            From your workspace&rsquo;s own resolved graph. Tare presents its implementation under the exact package names and versions
            your manifests ask for. <L href="/docs/build/substitution">Dependency substitution</L>
          </p>
        ),
      },
      {
        id: 'behaviour',
        title: 'Version-specific behaviour is kept',
        body: (
          <p>
            Where the lines differ, Tare differs with them: CPI return values keep their captured data on 1.x and their read-on-get
            behaviour on 0.x, and constraint errors carry the account name and compared keys on 1.x and the bare error on 0.x.
          </p>
        ),
      },
    ],
  },

  bytecode: {
    lead: 'After linking, tare-opt works on the sBPF program itself, as the validator will load it.',
    banner: [{ t: 'tare-opt', c: 'green' }],
    sections: [
      {
        id: 'stage',
        title: 'The stage',
        body: (
          <p>
            The front stage decides what code the Anchor language generates. The bytecode stage then reads the linked ELF, where code
            from every crate has been joined: its sections, symbols, relocations and instructions. It runs the real loader and verifier
            on the program before and after.
          </p>
        ),
      },
      {
        id: 'kept',
        title: 'What a rewrite must keep',
        body: (
          <List>
            <li>Section offsets, symbols and relocation sites.</li>
            <li>Syscall hashes and call addresses.</li>
            <li>Every possible target of an indirect call.</li>
          </List>
        ),
      },
      {
        id: 'reports',
        title: 'What it reports',
        body: (
          <p>
            The build prints the count of instructions rewritten and the size before and after. The full counters and both hashes are in{' '}
            <C>target/tare/optimization.json</C> and in the weighing. <L href="/docs/bytecode/passes">The passes</L>
          </p>
        ),
      },
    ],
  },

  'bytecode/passes': {
    lead: 'Each pass is a rewrite over the instruction stream that leaves every address where it was.',
    banner: [{ t: 'passes', c: 'line' }],
    sections: [
      {
        id: 'passes',
        title: 'The passes',
        body: (
          <Table
            head={['Pass', 'What it rewrites']}
            rows={[
              [<b key="1">Branch and call threading</b>, 'A jump or call to a thunk goes straight to the thunk\'s target.'],
              [<b key="2">Identity bypass</b>, 'Runs of instructions that leave every register as it was are stepped over.'],
              [<b key="3">Stack load forwarding</b>, 'A value just stored to the stack is used directly instead of loaded back.'],
              [<b key="4">Constant branches</b>, 'A branch whose outcome is fixed becomes a jump or falls through.'],
              [<b key="5">Strength reduction</b>, 'An arithmetic instruction becomes a cheaper one with the same result.'],
              [<b key="6">Unreachable code</b>, 'Blocks and functions no root can reach are neutralized.'],
            ]}
          />
        ),
      },
      {
        id: 'roots',
        title: 'Roots',
        body: (
          <p>
            The entrypoint, every direct call target and every relocation target are roots. Indirect calls keep every target they could
            reach.
          </p>
        ),
      },
      {
        id: 'counters',
        title: 'Counters',
        body: (
          <p>
            <C>optimization.json</C> holds one counter per pass, the program counters that were modified, the count of relocation sites
            preserved, and the SHA-256 of the program before and after.
          </p>
        ),
      },
    ],
  },

  'bytecode/checks': {
    lead: 'A rewrite is checked three times: by the verifier, in the VM, and by the proof.',
    banner: [{ t: 'checked', c: 'green' }],
    sections: [
      {
        id: 'verifier',
        title: 'The loader and verifier',
        body: <p>The optimized program is loaded and verified by the real Agave loader and verifier, the same as the program before it.</p>,
      },
      {
        id: 'vm',
        title: 'VM differential checks',
        body: (
          <p>
            Each rewrite rule runs against generated register and memory states and its boundary cases: whole registers, the stack, the
            heap, the input region, and errors. The original and the rewritten sequence must end the same.
          </p>
        ),
      },
      {
        id: 'proof',
        title: 'The proof',
        body: (
          <p>
            <C>tare prove</C> compares the final Tare program, after this stage, with the stock build. Everything the stage did is inside
            every proof and every record. <L href="/docs/prove">Prove</L>
          </p>
        ),
      },
    ],
  },

  registry: {
    lead: 'tare_registry is the one program Tare has on chain. It holds the records and the flags, and is itself built with tare build.',
    banner: [{ t: 'tare_registry', c: 'blue' }],
    sections: [
      {
        id: 'holds',
        title: 'What it holds',
        body: (
          <Table
            head={['Account', 'One per']}
            rows={[
              [<L key="1" href="/docs/registry/accounts">Config</L>, 'Registry'],
              [<L key="3" href="/docs/registry/accounts">Record</L>, 'Attested build'],
              [<L key="4" href="/docs/registry/accounts">Flag</L>, 'Filed input'],
            ]}
          />
        ),
      },
      {
        id: 'read',
        title: 'Reading it',
        body: (
          <p>
            Records are ordinary accounts any client, explorer or wallet can read next to a program&rsquo;s address. Every instruction
            emits an event, which is what the pages of <L href="/builds">the registry</L> are read from.
          </p>
        ),
      },
      {
        id: 'own',
        title: 'Its own weighing',
        body: (
          <p>
            The registry is an Anchor program, built with <C>tare build</C> like any other. Its weights and its proof are a record like
            any other.
          </p>
        ),
      },
    ],
  },

  'registry/accounts': {
    lead: 'The three accounts of tare_registry.',
    banner: [
      { t: 'Record', c: 'blue' },
      { t: 'Flag', c: 'red' },
    ],
    sections: [
      {
        id: 'config',
        title: 'Config',
        body: (
          <Table
            head={['Field', 'What it is']}
            rows={[
              ['admin', 'The key that can change the config.'],
              ['fee recipient', <span key="r">The account every attest fee is paid to: the {TOKEN.ticker} launch wallet. <L href="/docs/token">Token</L></span>],
              ['attest fee', 'In lamports.'],
            ]}
          />
        ),
      },
      {
        id: 'record',
        title: 'Record',
        body: (
          <Table
            head={['Field', 'What it is']}
            rows={[
              ['program', 'The address the build is for.'],
              ['stock build hash, tare build hash', 'SHA-256 of each binary.'],
              ['source repository, commit', 'Where the source is and which commit was built.'],
              ['toolchain version', 'Which tare made the build.'],
              ['corpus hash, counts', 'The corpus and its three counts.'],
              ['gross total, net total', 'The summed weights.'],
              ['report hash, location', 'The pinned proof report.'],
              ['attester, slot', 'Who wrote it, and when.'],
            ]}
          />
        ),
      },
      {
        id: 'flag',
        title: 'Flag',
        body: (
          <Table
            head={['Field', 'What it is']}
            rows={[
              ['record', 'The record it is filed against.'],
              ['input hash, location', 'The pinned diverging input.'],
              ['flagger, slot', 'Who filed it, and when.'],
            ]}
          />
        ),
      },
    ],
  },

  'registry/instructions': {
    lead: 'The four instructions of tare_registry, each with its event.',
    banner: [
      { t: 'attest', c: 'blue' },
      { t: 'flag', c: 'red' },
    ],
    sections: [
      {
        id: 'instructions',
        title: 'Instructions',
        body: (
          <Table
            head={['Instruction', 'Signer', 'What it does']}
            rows={[
              [<C key="1">init_config</C>, "The registry program's upgrade authority", 'Creates the config, naming the admin, the fee recipient and the attest fee.'],
              [<C key="2">set_config</C>, 'The admin', 'Sets the admin, the fee recipient and the attest fee. Passing another key as the admin hands the config over to it.'],
              [<C key="3">attest</C>, 'The attester', 'Writes a record and pays the fee from the attester to the fee recipient, in the same instruction.'],
              [<C key="4">flag</C>, 'The flagger', 'Writes a flag against a record.'],
            ]}
          />
        ),
      },
      {
        id: 'cli',
        title: 'From the CLI',
        body: (
          <p>
            You do not build these transactions by hand: <C>tare attest</C> sends <C>attest</C> and <C>tare flag</C> sends{' '}
            <C>flag</C>, each signed by the keypair you pass. The site sends none; it is read-only.
          </p>
        ),
      },
      {
        id: 'events',
        title: 'Events',
        body: <p>Each instruction emits an event carrying what it wrote. An indexer turns those into the registry&rsquo;s pages.</p>,
      },
    ],
  },

  token: {
    lead: `${TOKEN.ticker} is the token of the Tare registry. Every attested build reaches it through the attest fee.`,
    banner: [{ t: TOKEN.ticker, c: 'white' }],
    sections: [
      {
        id: 'facts',
        title: 'The token',
        body: (
          <Table
            head={['', '']}
            rows={[
              ['Ticker', TOKEN.ticker],
              ['Supply', `${units(TOKEN.supply)}, fixed`],
              ['Launch', TOKEN.launch],
              ['Fee asset', TOKEN.feeAsset],
              ...(TOKEN.mint ? [['Mint', <C key="m">{TOKEN.mint}</C>]] : []),
              ...(TOKEN.venue
                ? [
                    [
                      'Venue',
                      <a key="v" className="link" href={TOKEN.venue} target="_blank" rel="noreferrer">
                        {TOKEN.venue.replace(/^https?:\/\//, '')}
                      </a>,
                    ],
                  ]
                : []),
            ]}
          />
        ),
      },
      {
        id: 'launch',
        title: 'The launch',
        body: <p>{TOKEN.allocation}</p>,
      },
      {
        id: 'fee',
        title: 'What the attest fee does',
        body: (
          <>
            <p>
              Building, weighing and proving cost nothing. <C>tare attest</C> carries a fee in SOL, the asset a team already holds to
              deploy.
            </p>
            <Table
              head={['Step', 'What happens']}
              rows={[
                [<C key="1">tare attest</C>, 'A team publishes a proof. The same instruction pays the whole fee to the launch wallet.'],
                [<b key="2">Buy</b>, `On a fixed interval, the crank on Tare's server signs as the launch wallet and buys ${TOKEN.ticker} through Jupiter with a fixed amount of SOL.`],
                [<b key="3">Burn</b>, `It burns every ${TOKEN.ticker} that swap delivered: the quote's minimum in the swap's own transaction, the rest right after.`],
              ]}
            />
          </>
        ),
      },
      {
        id: 'registry',
        title: 'In the registry program',
        body: (
          <p>
            The attest fee and the fee recipient are fields of <C>Config</C>. <C>attest</C> refuses a fee recipient other than the one{' '}
            <C>Config</C> names. <L href="/docs/registry/accounts">Accounts</L>
          </p>
        ),
      },
    ],
  },
}
