import type { ProgramSample, RecordSummary } from '@tare/contract'
import type { Metadata } from 'next'
import Link from 'next/link'
import { AddressField } from '@/components/AddressField'
import { CommandLine, CopyButton } from '@/components/Copy'
import { Records } from '@/components/Records'
import { Resample } from '@/components/Resample'
import { Beam } from '@/components/Scale'
import { Chip, Page, PageHead, Panel, Regions, Section, Slot, Stat, Stats, State, type TitleBlock } from '@/components/ui'
import { api, read, scenario } from '@/lib/api'
import { instructionName } from '@/lib/bench'
import { LIMITS } from '@/lib/facts'
import { isAddress, percent, short, size, sol, stamp, units } from '@/lib/format'
import styles from './page.module.css'

type Props = { params: Promise<{ program: string }>; searchParams: Promise<{ fixture?: string }> }

export const dynamic = 'force-dynamic'

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { program } = await params
  return { title: `Weighing ${short(program)}`, description: `Compute by instruction, binary size and rent for the program ${program}.` }
}

export default async function WeighProgram({ params, searchParams }: Props) {
  const { program } = await params
  const { fixture } = await searchParams

  if (!isAddress(program)) {
    return (
      <Page>
        <PageHead title={[{ t: 'weigh', c: 'line' }, { t: 'a program', c: 'green' }]} />
        <State c="red" word="Not an address" title="That is not a Solana address" action={<AddressField />}>
          A program address is 32 to 44 base58 characters. Enter the address of a deployed program.
        </State>
      </Page>
    )
  }

  const result = await read(() => api.weigh(program, scenario(fixture)))

  if (result === null) {
    return (
      <Page>
        <Head program={program} />
        <State
          c="red"
          word="Not read"
          title="The weigh service did not answer"
          action={
            <Link href={`/weigh/${program}`} className="btn" data-kind="secondary">
              Try again
            </Link>
          }
        >
          This program&rsquo;s transactions could not be sampled just now. Nothing about the program changed.
        </State>
        <FromSource program={program} />
      </Page>
    )
  }

  if (result.status === 'not_program') {
    return (
      <Page>
        <Head program={program} state={{ t: 'not a program', c: 'red' }} />
        <State c="red" word="Not a program" title="This address is not a program" action={<AddressField />}>
          The account at this address is not executable, so it has no instructions to weigh. Enter the address of a deployed program.
        </State>
      </Page>
    )
  }

  if (result.status === 'sampling') {
    return (
      <Page>
        <Head program={program} state={{ t: 'on the scale', c: 'yellow' }} />
        <State c="yellow" word="On the scale" title="Sampling this program&rsquo;s recent transactions">
          Its recent mainnet transactions are being read and grouped by instruction. This page fills in by itself.
          <Resample seconds={Math.max(result.retryAfterSeconds, 2)} />
        </State>
        <RecordsOf program={program} records={result.records} />
        <FromSource program={program} />
      </Page>
    )
  }

  const { sample, records } = result
  return (
    <Page wide>
      <Head program={program} sample={sample} />
      <Regions
        work={
          <>
            {sample.instructions.length > 0 && (
              <Slot order={2}>
                <Where sample={sample} />
              </Slot>
            )}
            <Slot order={3}>
              <Section title="Compute by instruction" id="compute-h" side={`Sampled ${stamp(sample.sampledAt)}`}>
                {result.status === 'calls_failed' ? (
                  <State c="red" word="Failing" title="Every recent call failed">
                    This program was called in the sampled window, but every call that ran it failed, so there is no compute to group.
                    {sample.signaturesFailed != null && sample.signaturesExamined != null && (
                      <>
                        {' '}
                        {units(sample.signaturesFailed)} of the {units(sample.signaturesExamined)} recent signatures read were failed transactions.
                      </>
                    )}{' '}
                    Its size and rent are read from the program account.
                  </State>
                ) : result.status === 'no_transactions' || sample.instructions.length === 0 ? (
                  <State word="Quiet" title="No recent transactions">
                    This program has not been called in the sampled window, so there is no compute to group. Its size and rent are read from
                    the program account.
                  </State>
                ) : (
                  <Panel flush>
                    <Sampled sample={sample} />
                  </Panel>
                )}
              </Section>
            </Slot>
            {records.length > 0 && (
              <Slot order={4}>
                <RecordsOf program={program} records={records} />
              </Slot>
            )}
          </>
        }
        aside={
          <>
            <Slot order={1}>
              <Stats>
                <Stat value={size(sample.binarySize)[0]} unit={size(sample.binarySize)[1]} label="Binary size" />
                <Stat value={sol(sample.rentLamports)} unit="SOL" label="Rent locked" />
                <Stat value={units(sample.instructions.length)} label="Instructions seen" />
              </Stats>
            </Slot>
            <Slot order={5}>
              <FromSource program={program} />
            </Slot>
          </>
        }
      />
    </Page>
  )
}

function Head({ program, sample, state }: { program: string; sample?: ProgramSample; state?: TitleBlock }) {
  return (
    <PageHead
      title={[{ t: 'program', c: 'line' }, state ?? { t: sample?.program.name ?? short(program, 6, 6), c: 'white' }]}
      lead={
        <span className={styles.address}>
          <span className="num">{program}</span>
          <CopyButton text={program} />
        </span>
      }
    >
      {sample && (
        <>
          <Chip c={sample.anchor ? 'white' : 'line'}>{sample.anchor ? 'Anchor program' : 'Not an Anchor program'}</Chip>
          <Chip>{sample.hasIdl ? 'IDL on chain' : 'No IDL on chain'}</Chip>
        </>
      )}
    </PageHead>
  )
}

/** Where the program's compute goes: one beam split by instruction, heaviest first, and its largest parts named. */
function Where({ sample }: { sample: ProgramSample }) {
  const sorted = [...sample.instructions].sort((a, b) => b.share - a.share)
  const named = sorted.slice(0, 4)
  // Past 16 parts the beam would outgrow a phone; the lightest instructions share one part.
  const parts = sorted.slice(0, 16).map((ix) => ({ share: ix.share, c: 'ink' as const, label: instructionName(ix) }))
  if (sorted.length > 16) {
    const rest = sorted.slice(15)
    parts[15] = { share: rest.reduce((s, ix) => s + ix.share, 0), c: 'ink', label: `${rest.length} more` }
  }
  return (
    <Section title="Where its compute goes" id="where-h" side={`${units(sample.transactions)} transactions sampled`}>
      <Beam tall parts={parts} />
      <dl className={styles.legend}>
        {named.map((ix, i) => (
          <div key={ix.discriminator} style={{ opacity: Math.max(1 - i * 0.13, 0.5) }}>
            <dd className="num">{percent(ix.share, 1)}</dd>
            <dt className="num">{instructionName(ix)}</dt>
          </div>
        ))}
        {sorted.length > named.length && (
          <div className={styles.rest}>
            <dd className="num">{percent(sorted.slice(named.length).reduce((s, ix) => s + ix.share, 0), 1)}</dd>
            <dt>{sorted.length - named.length} more</dt>
          </div>
        )}
      </dl>
    </Section>
  )
}

function Sampled({ sample }: { sample: ProgramSample }) {
  const sorted = [...sample.instructions].sort((a, b) => b.share - a.share)
  const max = Math.max(...sorted.map((i) => i.share), 0.0001)
  return (
    <div className="table-scroll">
      <table className="table">
        <caption className="sr-only">Compute units by instruction from sampled mainnet transactions</caption>
        <thead>
          <tr>
            <th scope="col">Instruction</th>
            <th scope="col" className={styles.barCol}>
              Share of the program&rsquo;s compute
            </th>
            <th scope="col" className="r">
              Share
            </th>
            <th scope="col" className="r">
              Calls
            </th>
            <th scope="col" className="r">
              Median
            </th>
            <th scope="col" className="r">
              p90
            </th>
            <th scope="col" className="r">
              Of a transaction
            </th>
          </tr>
        </thead>
        <tbody>
          {sorted.map((ix, i) => (
            <tr key={ix.discriminator} style={{ '--row': i } as React.CSSProperties}>
              <th scope="row" className={styles.ix}>
                {instructionName(ix)}
                {ix.name === null && <span className={styles.unknown}>unnamed</span>}
              </th>
              <td className={styles.barCol}>
                <span className={styles.bar} style={{ width: `${Math.max((ix.share / max) * 100, 1)}%` }} aria-hidden="true" />
              </td>
              <td className="num r">{percent(ix.share, 1)}</td>
              <td className="num r">{units(ix.calls)}</td>
              <td className={`num r ${styles.median}`}>{units(ix.median)}</td>
              <td className="num r">{units(ix.p90)}</td>
              <td className="num r">{((ix.p90 / LIMITS.transactionUnits) * 100).toFixed(1)}%</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

function RecordsOf({ program, records }: { program: string; records: RecordSummary[] }) {
  if (records.length === 0) return null
  return (
    <Section
      title="On the registry"
      id="records-h"
      side={
        <Link href={`/builds?program=${program}`} className="link">
          All records for this program
        </Link>
      }
    >
      <Panel flush>
        <Records items={records} showProgram={false} />
      </Panel>
    </Section>
  )
}

/** The commands that weigh this program exactly, from its source. */
function FromSource({ program }: { program: string }) {
  return (
    <Section
      title="Weigh it exactly, from source"
      id="source-h"
      side={
        <Link href="/docs/install" className="link">
          Install Tare
        </Link>
      }
    >
      <p className="t-body">
        This page reads what the deployed build burns. In the program&rsquo;s Anchor workspace, three commands give gross, tare and net
        for every instruction, and a proof that includes this program&rsquo;s own mainnet transactions.
      </p>
      <div className={styles.commands}>
        <CommandLine text="tare build" />
        <CommandLine text="tare weigh" />
        <CommandLine text={`tare prove --program ${program}`} />
      </div>
    </Section>
  )
}
