import type { Metadata } from 'next'
import Link from 'next/link'
import { CommandLine, CopyButton } from '@/components/Copy'
import { Scale } from '@/components/Scale'
import { Chip, Facts, Page, PageHead, Panel, Section, State } from '@/components/ui'
import { Weighing } from '@/components/Weighing'
import { api, ApiClientError, scenario } from '@/lib/api'
import { rows } from '@/lib/bench'
import { COMPARED, CORPORA } from '@/lib/facts'
import { isAddress, kb, short, sol, stamp, units } from '@/lib/format'
import styles from '../page.module.css'

type Props = { params: Promise<{ record: string }>; searchParams: Promise<{ fixture?: string }> }

export const dynamic = 'force-dynamic'

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { record } = await params
  return { title: `Record ${short(record)}`, description: `The attested Tare build at registry record ${record}: weights, corpus and the check line.` }
}

function Hash({ value }: { value: string }) {
  return (
    <span className={`${styles.hashes} ${styles.copy}`}>
      <span className="num">{value}</span>
      <CopyButton text={value} />
    </span>
  )
}

export default async function RecordPage({ params, searchParams }: Props) {
  const { record } = await params
  const { fixture } = await searchParams

  const notFound = (
    <Page>
      <PageHead title={[{ t: 'record', c: 'blue' }, { t: 'not found', c: 'red' }]} />
      <State
        c="red"
        word="Not found"
        title="No record at this address"
        action={
          <Link href="/builds" className="btn" data-kind="secondary">
            The registry
          </Link>
        }
      >
        <span className="num">{short(record, 8, 8)}</span> is not a record on the Tare registry. A record&rsquo;s address is printed by{' '}
        <code className="t-data">tare attest</code>.
      </State>
    </Page>
  )
  if (!isAddress(record)) return notFound

  let detail
  try {
    detail = await api.record(record, scenario(fixture))
  } catch (e) {
    if (e instanceof ApiClientError && (e.code === 'RECORD_NOT_FOUND' || e.code === 'NOT_FOUND' || e.code === 'VALIDATION_ERROR')) return notFound
    return (
      <Page>
        <PageHead title={[{ t: 'record', c: 'blue' }, { t: 'not read', c: 'red' }]} />
        <State
          c="red"
          word="Not read"
          title="The registry did not answer"
          action={
            <Link href={`/builds/${record}`} className="btn" data-kind="secondary">
              Try again
            </Link>
          }
        >
          This record could not be read just now. It is on chain and unchanged.
        </State>
      </Page>
    )
  }

  const r = detail.record
  const report = detail.report
  const name = r.program.name ?? (r.program.address ? short(r.program.address) : 'Unnamed program')

  return (
    <Page>
      <PageHead
        title={[{ t: 'record', c: 'blue' }, { t: name, c: 'white' }]}
        lead={
          <>
            Record <span className="num">{short(r.address, 6, 6)}</span>, attested {stamp(r.createdAt)} at slot{' '}
            <span className="num">{units(r.slot)}</span>.
          </>
        }
      >
        {r.flags > 0 ? (
          <Chip c="red">
            {r.flags} {r.flags === 1 ? 'flag' : 'flags'}
          </Chip>
        ) : (
          <Chip c="green">No flags</Chip>
        )}
        {r.program.address && (
          <Link href={`/weigh/${r.program.address}`} className="btn" data-kind="secondary" data-size="sm">
            Weigh the deployed program
          </Link>
        )}
      </PageHead>

      <Scale gross={r.gross} net={r.net} notes={['stock build, compute units', "the framework’s own weight", 'Tare build, compute units']} />

      <Section
        title="Check this record"
        id="check-h"
        side={
          <Link href="/docs/check" className="link">
            What the check does
          </Link>
        }
      >
        <p className="t-body">
          Rebuilds the source at the record&rsquo;s commit in the pinned container, reruns the proof and compares the hashes. It
          reports <code className="t-data">matches</code> or the first difference.
        </p>
        <CommandLine text={detail.checkCommand} />
      </Section>

      <Section title="Weights by instruction" id="weights-h" side={report ? `From the pinned report ${short(r.reportHash, 6, 6)}` : undefined}>
        {report ? (
          <Panel>
            <Weighing rows={rows(report)} totals={report.totals} caption={`${name}: gross, tare and net compute units by instruction`} />
          </Panel>
        ) : (
          <State c="yellow" word="Report unavailable" title="The pinned report could not be fetched">
            The record on chain holds the totals above and the report&rsquo;s hash. The per-instruction weights are in the pinned report at{' '}
            <span className="num">{r.reportLocation}</span>, which did not answer. The check line rebuilds them from source.
          </State>
        )}
      </Section>

      <div className={styles.two}>
        <Section title="The corpus" id="corpus-h">
          <Panel>
            <dl className={styles.corpus}>
              {CORPORA.map((c) => (
                <div key={c.key}>
                  <dd className="num">{units(r.corpus.counts[c.key])}</dd>
                  <dt className="t-small">{c.label}</dt>
                </div>
              ))}
            </dl>
            <Facts
              rows={[
                { label: 'Corpus hash', value: <Hash value={r.corpus.hash} /> },
                ...(report
                  ? [
                      {
                        label: 'Proof',
                        value:
                          report.proof.status === 'matched' ? (
                            <Chip c="green">Every comparison matched</Chip>
                          ) : (
                            <Chip c="red">Diverged</Chip>
                          ),
                      },
                      { label: 'Compared', value: `${COMPARED.join(', ')} after each of ${units(report.proof.comparisons.inputs)} inputs` },
                    ]
                  : []),
              ]}
            />
          </Panel>
        </Section>

        <Section title="The builds" id="builds-h">
          <Panel>
            <Facts
              rows={[
                ...(r.program.address ? [{ label: 'Program', value: <Hash value={r.program.address} /> }] : []),
                { label: 'Stock build', value: <Hash value={r.stockBuildHash} /> },
                { label: 'Tare build', value: <Hash value={r.tareBuildHash} /> },
                ...(report
                  ? [
                      {
                        label: 'Binary size',
                        value: (
                          <span className="num">
                            {kb(report.builds.stock.sizeBytes)} to {kb(report.builds.tare.sizeBytes)}
                          </span>
                        ),
                      },
                      {
                        label: 'Rent locked',
                        value: (
                          <span className="num">
                            {sol(report.builds.stock.rentLamports)} to {sol(report.builds.tare.rentLamports)} SOL
                          </span>
                        ),
                      },
                    ]
                  : []),
                {
                  label: 'Source',
                  value: r.source.repository ? (
                    <a className="link" href={r.source.repository} target="_blank" rel="noreferrer">
                      {r.source.repository.replace(/^https?:\/\//, '')}
                    </a>
                  ) : (
                    'Not stated'
                  ),
                },
                { label: 'Commit', value: r.source.commit ? <Hash value={r.source.commit} /> : 'Not stated' },
                { label: 'Toolchain', value: <span className="num">{r.toolchainVersion}</span> },
                { label: 'Attester', value: <Hash value={r.attester} /> },
              ]}
            />
          </Panel>
        </Section>
      </div>

      <Section
        title="Flags"
        id="flags-h"
        side={
          <Link href="/docs/check" className="link">
            Filing a flag
          </Link>
        }
      >
        {detail.flags.length === 0 ? (
          <State c="green" word="None" title="No input has been filed against this record">
            A flag is an input on which the stock build and the Tare build differ, confirmed locally and filed on chain with{' '}
            <code className="t-data">tare flag</code>.
          </State>
        ) : (
          <div className={styles.flags}>
            {detail.flags.map((f) => (
              <Panel key={f.address}>
                <Facts
                  rows={[
                    { label: 'Flag', value: <Hash value={f.address} /> },
                    { label: 'Filed', value: `${stamp(f.createdAt)}, slot ${units(f.slot)}` },
                    { label: 'Flagger', value: <Hash value={f.flagger} /> },
                    { label: 'Input hash', value: <Hash value={f.inputHash} /> },
                    { label: 'Input', value: <span className="num">{f.inputLocation}</span> },
                    ...(f.input
                      ? [
                          { label: 'Input from', value: `${f.input.divergence.input.label} (${f.input.divergence.input.kind})` },
                          {
                            label: 'Differs in',
                            value: (
                              <span className={styles.hashes}>
                                {f.input.divergence.differences.map((d, i) => (
                                  <Chip key={i} c="red">
                                    {d.kind.replace('_', ' ')}
                                  </Chip>
                                ))}
                              </span>
                            ),
                          },
                        ]
                      : []),
                  ]}
                />
              </Panel>
            ))}
          </div>
        )}
      </Section>
    </Page>
  )
}
