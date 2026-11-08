import Link from 'next/link'
import { AddressField } from '@/components/AddressField'
import { B, type Colour } from '@/components/Block'
import { CommandLine } from '@/components/Copy'
import { Scale } from '@/components/Scale'
import { Bar, Weighing, WeighingPending } from '@/components/Weighing'
import { bench, featured, rows } from '@/lib/bench'
import { COMMANDS, COMPARED, CORPORA, IDENTICAL, INSTALL, TOKEN } from '@/lib/facts'
import { kb, percent, units } from '@/lib/format'

const at = (i: number) => ({ '--i': i }) as React.CSSProperties
import styles from './Bands.module.css'

const KIND = { anchor_test: "one of Anchor’s own test programs", anchor_example: "one of Anchor’s own examples", production: 'a production program' } as const

/** Under the first view: what Tare is in one line of the hero's own texture, and the way in. */
export function StartBand() {
  return (
    <section className={`${styles.band} ${styles.start}`} aria-labelledby="start-h" data-reveal>
      <div className={`wrap ${styles.centre}`}>
        <h2 id="start-h" className={styles.line} data-r="pop">
          <B c="line">anchor build</B>
          <span className={styles.arrow} role="img" aria-label="becomes">
            <svg viewBox="0 0 40 24" aria-hidden="true">
              <path d="M2 12h34M26 3l10 9-10 9" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
          </span>
          <B c="green">tare build</B>
        </h2>
        <p className={styles.say} data-r style={at(2)}>
          The Anchor program you already wrote comes out at hand-written compute cost. Not a line changed, and a proof on chain that it
          is the same program.
        </p>
        <div className={styles.way} data-r style={at(4)}>
          <AddressField size="lg" />
          <div className={styles.row}>
            {INSTALL ? (
              <CommandLine text={INSTALL} />
            ) : (
              <Link href="/docs/install" className="link t-small">
                Install Tare
              </Link>
            )}
            <Link href="/docs/build" className="link t-small">
              How the build works
            </Link>
          </div>
        </div>
      </div>
    </section>
  )
}

/** The centre: one real program on the scale, its three weights as the largest thing on the page. */
export function WeighingBand() {
  const p = featured
  if (!p) {
    return (
      <section className={styles.band} aria-labelledby="weighing-h">
        <div className={`wrap ${styles.centre}`}>
          <h2 id="weighing-h" className={styles.kicker}>
            Three weights for every instruction
          </h2>
          <div className={styles.open}>
            <WeighingPending note="The benchmark’s first corpus program is being built and weighed. Its table lands here." />
          </div>
        </div>
      </section>
    )
  }
  const t = p.weighing.totals
  const all = rows(p.weighing)
  return (
    <section className={styles.band} aria-labelledby="weighing-h" data-reveal>
      <div className={`wrap ${styles.centre}`}>
        <h2 id="weighing-h" className={styles.kicker} data-r>
          <b>{p.name}</b>, {p.name === 'bench' ? "Anchor’s own benchmark program" : KIND[p.kind]}, on the scale
        </h2>
        <dl className={styles.scale} style={{ '--chars': units(t.gross).length + units(t.tare).length + units(t.net).length } as React.CSSProperties}>
          <div data-r style={at(1)}>
            <dt>
              <B c="line">Gross</B>
            </dt>
            <dd className="num">{units(t.gross)}</dd>
            <dd className={styles.note}>built with stock Anchor</dd>
          </div>
          <div className={styles.tareCol} data-r style={at(9)}>
            <dt>
              <B c="magenta">Tare</B>
            </dt>
            <dd className="num">{units(t.tare)}</dd>
            <dd className={styles.note}>the framework&rsquo;s own weight</dd>
          </div>
          <div className={styles.netCol} data-r style={at(3)}>
            <dt>
              <B c="green">Net</B>
            </dt>
            <dd className="num" data-count data-from={t.gross} data-to={t.net}>
              {units(t.net)}
            </dd>
            <dd className={styles.note}>built with Tare</dd>
          </div>
        </dl>
        <div className={styles.beam} aria-hidden="true" data-r="fade" style={at(3)}>
          <Bar gross={t.gross} net={t.net} max={t.gross} />
        </div>
        <p className={styles.unit} data-r="fade" style={at(14)}>
          Compute units for one call of each of its {all.length} instructions. <span className={styles.tareText}>{percent(t.tare, t.gross)}</span> of
          the gross was tare.
        </p>
        <div className={styles.open} data-r style={at(8)}>
          <Weighing rows={all} totals={t} limit={8} caption={`${p.name}: gross, tare and net compute units by instruction`} />
        </div>
        <p className={`t-small ${styles.under}`}>
          {p.source.repository ? (
            <a className="link" href={p.source.repository} target="_blank" rel="noreferrer">
              Its public source
            </a>
          ) : (
            'Its public source'
          )}
          {p.source.commit && (
            <>
              {' '}
              at <span className="num">{p.source.commit.slice(0, 7)}</span>
            </>
          )}
          , built unchanged.{' '}
          {p.weighing.builds.tare.sizeBytes < p.weighing.builds.stock.sizeBytes && (
            <>
              Binary <span className="num">{kb(p.weighing.builds.stock.sizeBytes)}</span> to{' '}
              <span className={`num ${styles.netText}`}>{kb(p.weighing.builds.tare.sizeBytes)}</span>.{' '}
            </>
          )}
          <Link href="/bench" className="link">
            {all.length > 8 ? `All ${all.length} instructions and every corpus program` : 'Every corpus program'}
          </Link>
        </p>
      </div>
    </section>
  )
}

const COMMAND_COLOUR: Colour[] = ['green', 'magenta', 'cyan', 'blue']

/** The four commands as four lines of a terminal, set at the size of a headline. */
export function CommandsBand() {
  return (
    <section className={styles.band} aria-labelledby="commands-h" data-reveal>
      <div className={`wrap ${styles.centre}`}>
        <h2 id="commands-h" className={styles.kicker}>
          Four commands
        </h2>
        <ol className={styles.term}>
          {COMMANDS.map((c, i) => (
            <li key={c.cmd} data-r style={at(i * 3)}>
              <code>
                <span className={styles.prompt} aria-hidden="true">
                  $
                </span>
                <B c={COMMAND_COLOUR[i]}>{c.cmd}</B>
              </code>
              <span className={styles.comment}>{c.does}</span>
            </li>
          ))}
        </ol>
      </div>
    </section>
  )
}

const SAME: Colour[] = ['green', 'cyan', 'white', 'magenta', 'yellow', 'blue', 'green']

/** What is identical between the two builds: the brand holds this view by itself. */
/** The seven as three rows of near-equal length, each stretched to the same width. */
const SAME_ROWS = [
  [0, 1],
  [3, 5],
  [2, 4, 6],
]

export function IdenticalBand() {
  return (
    <section className={styles.band} aria-labelledby="identical-h" data-reveal>
      <div className={`wrap ${styles.centre}`}>
        <h2 id="identical-h" className={styles.kicker}>
          What stays identical
        </h2>
        <ul className={styles.same}>
          {SAME_ROWS.flatMap((row, r) => [
            ...row.map((i) => (
              <li key={IDENTICAL[i]} data-r="pop" style={at(i)}>
                <B c={SAME[i % SAME.length]}>{IDENTICAL[i]}</B>
              </li>
            )),
            <li key={`row-${r}`} className={styles.sameBreak} aria-hidden="true" />,
          ])}
        </ul>
        <p className={styles.say} data-r style={at(9)}>
          Clients, indexers and every account already on chain keep working. The Tare build upgrades the program in place, at the same
          address.
        </p>
      </div>
    </section>
  )
}

/** How the proof works, as one open figure: every input through both builds, five results that must be equal. */
export function ProofBand() {
  return (
    <section className={styles.band} aria-labelledby="proof-h" data-reveal>
      <div className={`wrap ${styles.centre}`}>
        <h2 id="proof-h" className={styles.kicker}>
          How the proof works
        </h2>
        <figure className={styles.proof} aria-label="Every input runs on the stock build and the Tare build; five results are compared">
          <div className={styles.feed}>
            {CORPORA.map((c) => (
              <B key={c.key} c="cyan">
                {c.label}
              </B>
            ))}
          </div>
          <p className={styles.caption}>Every input from the three corpora runs on both builds.</p>
          <div className={styles.lanes}>
            <span className={styles.laneHead}>
              <B c="line">stock build</B>
            </span>
            <span />
            <span className={styles.laneHead}>
              <B c="green">tare build</B>
            </span>
            {COMPARED.map((c, i) => (
              <div key={c} className={styles.laneRow}>
                <span className={styles.left} data-r="fade" style={at(i * 3)}>
                  {c.toLowerCase()}
                </span>
                <span className={styles.eq} role="img" aria-label="must equal" data-r="fade" style={at(i * 3 + 4)}>
                  ==
                </span>
                <span className={styles.right} data-r="fade" style={at(i * 3)}>
                  {c.toLowerCase()}
                </span>
              </div>
            ))}
          </div>
          <p className={styles.caption}>
            One difference fails the proof and prints the input. All equal, and <code className={styles.inline}>tare attest</code> writes the
            build hashes, the source commit, the corpus and the weights to a record on chain.
          </p>
          <div className={styles.feed}>
            <B c="blue">record</B>
            <B c="line">tare check</B>
            <B c="line">tare flag</B>
          </div>
        </figure>
        <Link href="/docs/prove" className="link t-small">
          The three corpora and what is compared
        </Link>
      </div>
    </section>
  )
}

export type RegistryCount = { records: number } | null

/** The close: the whole corpus on one scale, and the address field once more. */
export function CloseBand({ registry }: { registry: RegistryCount }) {
  const t = bench.totals
  const weighed = t && t.weights.gross > 0

  return (
    <section className={`${styles.band} ${styles.close}`} aria-labelledby="close-h" data-reveal>
      <div className={`wrap ${styles.centre}`}>
        {weighed && (
          <>
            <p className={styles.kicker} data-r>
              <b>{units(t.measured)}</b> open-source programs, <b>{units(t.instructions)}</b> instructions, built unchanged
            </p>
            <div className={styles.corpus} data-r style={at(2)}>
              <Scale
                align="centre"
                gross={t.weights.gross}
                net={t.weights.net}
                notes={['every measured instruction, stock', "the framework’s own weight", 'the same instructions, built with Tare']}
              />
            </div>
            <div className={styles.row} data-r="fade" style={at(8)}>
              <Link href="/bench" className="link t-small">
                The benchmark
              </Link>
              {registry && registry.records > 0 && (
                <Link href="/builds" className="link t-small">
                  {units(registry.records)} {registry.records === 1 ? 'record' : 'records'} on the registry
                </Link>
              )}
            </div>
          </>
        )}
        <h2 id="close-h" className={`${styles.line} ${styles.closeLine}`}>
          <B c="line">weigh</B>
          <B c="green">any program</B>
        </h2>
        <p className={styles.say}>
          Its compute by instruction from recent mainnet transactions, its size and the rent it locks. By address, with no install.
        </p>
        <div className={styles.way}>
          <AddressField size="lg" />
        </div>
        <p className={styles.token}>
          <B c="white">{TOKEN.ticker}</B>
          <span>The attest fee buys {TOKEN.ticker} and burns it.</span>
          <Link href="/docs/token" className="link">
            The token
          </Link>
        </p>
      </div>
    </section>
  )
}
