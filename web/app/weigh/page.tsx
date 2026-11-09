import type { Metadata } from 'next'
import Link from 'next/link'
import { AddressField } from '@/components/AddressField'
import { B } from '@/components/Block'
import { Beam } from '@/components/Scale'
import { Chip, Page, Section, State } from '@/components/ui'
import { api, read, scenario } from '@/lib/api'
import { instructionName } from '@/lib/bench'
import { EXAMPLE_PROGRAMS } from '@/lib/facts'
import { kb, percent, short, stamp, units } from '@/lib/format'
import styles from './page.module.css'

export const metadata: Metadata = {
  title: 'Weigh a program',
  description: 'Weigh any deployed Solana program by its address: compute by instruction from recent mainnet transactions, binary size and rent.',
}
export const dynamic = 'force-dynamic'

export default async function WeighPage({ searchParams }: { searchParams: Promise<{ fixture?: string }> }) {
  const recent = await read(async () => api.recentWeighings({ limit: 24 }, scenario((await searchParams).fixture)))

  return (
    <Page>
      {/* The page is one action: the address, on the centre of the view. */}
      <header className={styles.stage}>
        <h1 className={styles.title} data-land>
          <B c="line">weigh</B>
          <B c="green">a program</B>
        </h1>
        <p className={styles.lead}>
          Any deployed program, by its address. Its compute by instruction from recent mainnet transactions, its binary size and the
          rent it locks.
        </p>
        <AddressField size="lg" autoFocus />
        <p className={styles.examples}>
          <span>Or open one</span>
          {EXAMPLE_PROGRAMS.map((p) => (
            <Link key={p.address} href={`/weigh/${p.address}`} className="btn" data-kind="secondary" data-size="sm">
              {p.name}
            </Link>
          ))}
        </p>
      </header>

      <Section title="Recently weighed" id="recent-h" side={recent && recent.items.length > 0 ? `As of ${stamp(recent.asOf)}` : undefined}>
        {recent === null ? (
          <State c="red" word="Not read" title="The weigh service did not answer">
            The list of recently weighed programs could not be read. Weighing by address above still goes to the program&rsquo;s page.
          </State>
        ) : recent.items.length === 0 ? (
          <State word="Empty" title="No program has been weighed yet">
            The first address entered above starts this list.
          </State>
        ) : (
          <ul className={styles.list}>
            {recent.items.map((s, n) => {
              const sorted = [...s.instructions].sort((a, b) => b.share - a.share)
              const top = sorted[0]
              const address = s.program.address
              const row = (
                <>
                  <span className={styles.name}>{s.program.name ?? (address ? short(address, 6, 6) : 'Unnamed program')}</span>
                  <span className={styles.kind}>
                    <Chip>{s.anchor ? 'Anchor' : 'Not Anchor'}</Chip>
                  </span>
                  <span className={styles.beam}>
                    {sorted.length > 0 ? (
                      <Beam parts={sorted.slice(0, 8).map((ix) => ({ share: ix.share, c: 'ink' as const, label: instructionName(ix) }))} />
                    ) : (
                      <span className={styles.quiet}>{s.signaturesFailed ? 'Every recent call failed' : 'No recent transactions'}</span>
                    )}
                  </span>
                  <span className={styles.top}>
                    {top && (
                      <>
                        <span className="num">{instructionName(top)}</span>
                        <span className={styles.quiet}>
                          {percent(top.share, 1)} of its compute, <span className="num">{units(top.median)}</span> units
                        </span>
                      </>
                    )}
                  </span>
                  <span className={`num ${styles.size}`}>{kb(s.binarySize)}</span>
                </>
              )
              return (
                <li key={address ?? s.sampledAt} style={{ '--row': n } as React.CSSProperties}>
                  {address ? (
                    <Link href={`/weigh/${address}`} className={styles.row}>
                      {row}
                    </Link>
                  ) : (
                    <span className={styles.row}>{row}</span>
                  )}
                </li>
              )
            })}
          </ul>
        )}
      </Section>
    </Page>
  )
}
