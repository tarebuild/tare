// The token's figures and burns, read from GET /v1/token, for /docs/token. Each part is a server component
// product-docs places in the chapter; the page re-reads the API at most once a minute.

import { open, TokenFacts, type Burn } from '@tare/contract'
import { Chip, State } from '@/components/ui'
import { API_URL } from '@/lib/api'
import { TOKEN } from '@/lib/facts'
import { short, sol, stamp, units } from '@/lib/format'
import styles from './docs.module.css'

const txUrl = (signature: string) => `https://solscan.io/tx/${signature}`

async function readToken(): Promise<TokenFacts | null> {
  try {
    const res = await fetch(`${API_URL}/v1/token`, { next: { revalidate: 60 } })
    if (!res.ok) return null
    const parsed = open(TokenFacts).safeParse(await res.json())
    return parsed.success ? parsed.data : null
  } catch {
    return null
  }
}

const notRead = (
  <State c="red" word="Not read" title="The token’s figures did not load">
    The API did not answer just now. Every figure here is also on chain, in the crank’s transactions.
  </State>
)

const tare = (n: number) => units(n)

/** Fees in, SOL spent and TARE burned, for attest fees and trading fees side by side and in total. */
export async function TokenFigures() {
  const t = await readToken()
  if (!t) return notRead
  const { attest, trading } = t.bySource
  const attestIn = attest.feesReceivedLamports
  const tradingIn = trading.feesReceivedLamports
  const rows: { label: string; unit: string; attest: string; trading: string; total: string }[] = [
    {
      label: 'Fees received',
      unit: 'SOL',
      attest: sol(attestIn),
      trading: sol(tradingIn),
      total: sol(attestIn + tradingIn),
    },
    { label: 'Kept by the claimer', unit: 'SOL', attest: '', trading: sol(trading.keptLamports), total: sol(trading.keptLamports) },
    { label: 'Spent buying', unit: 'SOL', attest: sol(attest.spentLamports), trading: sol(trading.spentLamports), total: sol(t.spentLamports) },
    { label: 'Burned', unit: TOKEN.ticker, attest: tare(attest.burned), trading: tare(trading.burned), total: tare(t.burned) },
  ]
  return (
    <div className={`table-scroll ${styles.tableWrap} ${styles.figures}`}>
      <table className="table">
        <caption className="sr-only">{TOKEN.ticker} fees received, SOL spent and tokens burned, by what paid for them</caption>
        <thead>
          <tr>
            <th scope="col" />
            <th scope="col" className="r">
              Attest fees
            </th>
            <th scope="col" className="r">
              Trading fees
            </th>
            <th scope="col" className="r">
              Total
            </th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => (
            <tr key={r.label} data-burned={r.unit === TOKEN.ticker || undefined}>
              <th scope="row">
                {r.label} <span className={styles.unit}>{r.unit}</span>
              </th>
              <td className="num r">{r.attest || <span className={styles.none}>none</span>}</td>
              <td className="num r">{r.trading}</td>
              <td className="num r">{r.total}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

/** The burned total beside the mint's supply, so the one can be checked against the drop in the other. */
export async function TokenSupply() {
  const t = await readToken()
  if (!t) return notRead
  const now = t.mintSupply
  return (
    <dl className={styles.supply}>
      <div>
        <dt>Supply at launch</dt>
        <dd className="num">{units(t.supply)}</dd>
      </div>
      <div data-c="magenta">
        <dt>Burned by the crank</dt>
        <dd className="num">{units(t.burned)}</dd>
      </div>
      {now != null && (
        <div data-c="green">
          <dt>The mint&rsquo;s supply now</dt>
          <dd className="num">{units(now)}</dd>
        </div>
      )}
      {now != null && (
        <div className={styles.check}>
          <dt>{Math.abs(t.supply - now - t.burned) < 0.001 ? 'Supply at launch less supply now equals the burned total' : 'Supply at launch less supply now'}</dt>
          <dd className="num">
            {units(t.supply)} &minus; {units(now)} = {units(t.supply - now)}
          </dd>
        </div>
      )}
      {t.mint && (
        <div>
          <dt>Mint</dt>
          <dd>
            <a className="link num" href={`https://solscan.io/token/${t.mint}`} target="_blank" rel="noreferrer">
              {short(t.mint, 6, 6)}
            </a>
          </dd>
        </div>
      )}
    </dl>
  )
}

const KIND: Record<Burn['kind'], { word: string; c: 'green' | 'magenta' | 'line' }> = {
  claim: { word: 'Claim', c: 'line' },
  buy: { word: 'Buy', c: 'green' },
  burn: { word: 'Burn', c: 'magenta' },
}

function Tx({ signature }: { signature: string }) {
  return (
    <a className="link num" href={txUrl(signature)} target="_blank" rel="noreferrer">
      {short(signature, 5, 5)}
    </a>
  )
}

/** The crank's latest transactions, newest first, each linked to its transaction. */
export async function TokenBurns() {
  const t = await readToken()
  if (!t) return notRead
  if (t.burns.length === 0) {
    return (
      <State word="None yet" title="The crank has not run yet">
        Its first claim, buy and burn appear here with their transactions.
      </State>
    )
  }
  return (
    <div className={`table-scroll ${styles.tableWrap} ${styles.burns}`}>
      <table className="table">
        <caption className="sr-only">The crank&rsquo;s latest transactions, newest first</caption>
        <thead>
          <tr>
            <th scope="col">When, UTC</th>
            <th scope="col">Kind</th>
            <th scope="col">Paid by</th>
            <th scope="col" className="r">
              SOL
            </th>
            <th scope="col" className="r">
              Burned
            </th>
            <th scope="col">Transaction</th>
            <th scope="col">From claim</th>
          </tr>
        </thead>
        <tbody>
          {t.burns.map((b) => {
            const kind = KIND[b.kind] ?? { word: b.kind, c: 'line' as const }
            return (
              <tr key={b.signature}>
                <td className="num">{stamp(b.at).replace(' UTC', '')}</td>
                <td>
                  <Chip c={kind.c}>{kind.word}</Chip>
                </td>
                <td>{b.source === 'attest' ? 'Attest fees' : b.source === 'trading' ? 'Trading fees' : b.source}</td>
                <td className="num r">
                  {b.kind === 'claim' ? (
                    <>
                      {sol(b.claimedLamports)} claimed
                      <span className={styles.sub}>{sol(b.keptLamports)} kept</span>
                    </>
                  ) : b.spentLamports > 0 ? (
                    `${sol(b.spentLamports)} spent`
                  ) : (
                    <span className={styles.none}>none</span>
                  )}
                </td>
                <td className="num r">{b.burned > 0 ? tare(b.burned) : <span className={styles.none}>none</span>}</td>
                <td>
                  <Tx signature={b.signature} />
                </td>
                <td>{b.claimSignature && b.claimSignature !== b.signature ? <Tx signature={b.claimSignature} /> : null}</td>
              </tr>
            )
          })}
        </tbody>
      </table>
    </div>
  )
}
