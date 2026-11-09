import type { Metadata } from 'next'
import Link from 'next/link'
import { CommandLine } from '@/components/Copy'
import { Records } from '@/components/Records'
import { Page, PageHead, Panel, Stat, Stats, State } from '@/components/ui'
import { api, read, scenario } from '@/lib/api'
import { isAddress, percent, short, stamp, units } from '@/lib/format'
import { ProgramFilter } from './ProgramFilter'
import styles from './page.module.css'

export const metadata: Metadata = {
  title: 'The registry',
  description: 'Every attested Tare build: the program, the attester, the source commit, the weights and the corpus the proof was shown on.',
}
export const dynamic = 'force-dynamic'

type Search = { program?: string; cursor?: string; fixture?: string }

export default async function Builds({ searchParams }: { searchParams: Promise<Search> }) {
  const { program, cursor, fixture } = await searchParams
  const filter = program && isAddress(program) ? program : undefined
  const call = scenario(fixture)
  const [page, totals] = await Promise.all([
    read(() => api.records({ program: filter, cursor, limit: 24 }, call)),
    read(() => api.registryTotals(call)),
  ])

  return (
    <Page>
      <PageHead
        title={[{ t: 'the registry', c: 'blue' }]}
        lead="Every attested build, newest first. A record holds the two build hashes, the source commit, the weights and the corpus its proof was shown on."
      >
        <ProgramFilter value={filter ?? ''} />
      </PageHead>

      {totals && totals.records > 0 && !filter && (
        <Stats>
          <Stat value={units(totals.records)} label={totals.records === 1 ? 'Record' : 'Records'} c="blue" />
          <Stat value={units(totals.programs)} label={totals.programs === 1 ? 'Program' : 'Programs'} />
          <Stat value={percent(totals.tare, totals.gross)} label="Of attested gross was tare" c="magenta" />
          <Stat value={units(totals.corpus)} label="Inputs compared" />
        </Stats>
      )}

      {page === null ? (
        <State
          c="red"
          word="Not read"
          title="The registry did not answer"
          action={
            <Link href="/builds" className="btn" data-kind="secondary">
              Try again
            </Link>
          }
        >
          The records could not be read just now. They are on chain and unchanged.
        </State>
      ) : page.items.length === 0 ? (
        filter ? (
          <State
            word="Empty"
            title="No record for this program"
            action={
              <>
                <Link href={`/weigh/${filter}`} className="btn" data-kind="secondary">
                  Weigh it
                </Link>
                <Link href="/builds" className="link t-small">
                  All records
                </Link>
              </>
            }
          >
            <span className="num">{short(filter, 6, 6)}</span> has no attested build yet. Its team makes the first one from the
            program&rsquo;s workspace with <code className="t-data">tare attest</code>.
          </State>
        ) : (
          <State word="Empty" title="The first record is one command away" action={<CommandLine text="tare attest" />}>
            After <code className="t-data">tare build</code> and <code className="t-data">tare prove</code>, attesting writes the
            proof to a record on chain and prints its page here.{' '}
            <Link href="/docs/check" className="link">
              Attest and check
            </Link>
          </State>
        )
      ) : (
        <>
          <Panel flush>
            <Records items={page.items} />
          </Panel>
          <div className={styles.foot}>
            <span className="t-small">As of {stamp(page.asOf)}</span>
            {page.page.hasMore && page.page.nextCursor && (
              <Link
                className="btn"
                data-kind="secondary"
                data-size="sm"
                href={{ pathname: '/builds', query: { ...(filter ? { program: filter } : {}), cursor: page.page.nextCursor } }}
              >
                Older records
              </Link>
            )}
          </div>
        </>
      )}
    </Page>
  )
}
