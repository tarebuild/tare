import { fixtureReport } from '@tare/contract/fixtures'
import { AddressField } from '@/components/AddressField'
import { Ast, B, type Colour } from '@/components/Block'
import { Cascade } from '@/components/Cascade'
import { CommandLine } from '@/components/Copy'
import { Glass } from '@/components/Glass'
import { Lockup, Mark } from '@/components/Logo'
import { Chip, Page, PageHead, Panel, Section, Stat, Stats, State } from '@/components/ui'
import { Weighing, WeighingPending } from '@/components/Weighing'
import { rows } from '@/lib/bench'
import { COMMAND_STANZA, fill } from '@/lib/lines'
import { sourceFrom } from '@/lib/source'
import styles from './page.module.css'

const COLOURS: { c: Colour; name: string; role: string; hex: string }[] = [
  { c: 'green', name: 'Green', role: 'Main colour. The mark, the primary action, net, matched.', hex: '#00E531' },
  { c: 'magenta', name: 'Magenta', role: 'Tare: what was taken off.', hex: '#F04FE8' },
  { c: 'white', name: 'White', role: 'The neutral lead block.', hex: '#FFFFFF' },
  { c: 'yellow', name: 'Yellow', role: 'Pending: on the scale, sampling.', hex: '#FFE600' },
  { c: 'cyan', name: 'Cyan', role: 'Proof and corpus.', hex: '#19E3D2' },
  { c: 'blue', name: 'Blue', role: 'Records, on chain.', hex: '#2B2BFF' },
  { c: 'red', name: 'Red', role: 'Flags, failures, not found.', hex: '#F0241B' },
]


export default function Board() {
  return (
    <Page>
      <PageHead
        title={[{ t: 'tare', c: 'green' }, { t: 'the brand board', c: 'line' }]}
        lead="A block is a unit of weight, and a build takes blocks away. Black field, seven key colours as rounded blocks behind words, mono for anything typed, sentence case, soft corners, one piece of glass."
      >
        <Lockup height={22} />
      </PageHead>

      <Section title="The signature: written, lit, edited" id="sig-h">
        <div className={styles.stage}>
          <div className={styles.stream}>
            <Cascade stanzas={sourceFrom(3, 4)} amplitude={0} mode="build" className={styles.code} />
          </div>
          <h2 className={styles.line}>
            <Glass className={styles.glass}>Weigh</Glass>
            <B c="green">less.</B>
          </h2>
        </div>
      </Section>

      <Section title="The cascade in its wave, for films and images" id="wave-h">
        <Panel>
          <Cascade stanzas={fill([COMMAND_STANZA], 5)} amplitude={3} spread hold={90} delay={60} />
        </Panel>
      </Section>

      <Section title="Colour by role" id="colour-h">
        <div className={styles.swatches}>
          {COLOURS.map((k) => (
            <div key={k.c} className={styles.swatch}>
              <B c={k.c}>{k.name}</B>
              <span className="num t-small">{k.hex}</span>
              <span className="t-small">{k.role}</span>
            </div>
          ))}
        </div>
      </Section>

      <Section title="Type" id="type-h">
        <Panel>
          <div className={styles.type}>
            <p className="t-display">
              anchor build becomes <B c="green">tare build</B>
            </p>
            <p className="t-h2">Three weights for every instruction</p>
            <p className="t-h3">Compute by instruction</p>
            <p className="t-lead">The Anchor program you already wrote comes out at hand-written compute cost.</p>
            <p className="t-body">
              Each instruction runs on the stock build and the Tare build in a local SVM, over the same inputs. The weighing prints in
              the terminal and is written to a file.
            </p>
            <p className="t-data">tare prove --program whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc · 1,400,000 · 0123456789</p>
          </div>
        </Panel>
      </Section>

      <Section title="The mark and the blocks" id="mark-h">
        <Panel>
          <div className={styles.row}>
            <span style={{ color: 'var(--accent)' }}>
              <Mark size={56} />
            </span>
            <Lockup height={30} />
            <span className={styles.big}>
              <B c="line">Gross</B>
              <Ast />
              <B c="magenta">Tare</B>
              <Ast />
              <B c="green">Net</B>
            </span>
          </div>
        </Panel>
      </Section>

      <Section title="Actions, fields and chips" id="kit-h">
        <Panel>
          <div className={styles.row}>
            <button type="button" className="btn">
              Weigh
            </button>
            <button type="button" className="btn" data-kind="secondary">
              Install Tare
            </button>
            <button type="button" className="btn" data-size="sm">
              Install
            </button>
            <button type="button" className="btn" disabled>
              Weighing
            </button>
            <a className="link t-small" href="#kit-h">
              How the build works
            </a>
            <Chip>Anchor program</Chip>
            <Chip c="green">Proof matched</Chip>
            <Chip c="yellow">On the scale</Chip>
            <Chip c="red">1 flag</Chip>
            <Chip c="blue">Record</Chip>
            <Chip c="cyan">Corpus</Chip>
          </div>
          <div className={styles.fields}>
            <AddressField size="lg" />
            <CommandLine text="tare check FpspPVPCTPWadzmt2PhfyzVhNLAHhLEuLsTHjjihSQTr" />
          </div>
        </Panel>
      </Section>

      <Section title="Figures" id="fig-h">
        <Stats>
          <Stat value="4,663" label="Gross, compute units" />
          <Stat value="1,435" label="Tare" c="magenta" />
          <Stat value="3,228" label="Net" c="green" />
          <Stat value="181.4" unit="KB" label="Binary size" />
        </Stats>
      </Section>

      <Section title="The weighing" id="weighing-h" side="Sample values from the contract’s fixtures">
        <Panel>
          <Weighing rows={rows(fixtureReport)} totals={fixtureReport.totals} caption="A sample weighing" />
        </Panel>
      </Section>

      <Section title="States" id="states-h">
        <div className={styles.states}>
          <Panel>
            <WeighingPending rows={3} note="A weighing that is still being measured." />
          </Panel>
          <State word="Empty" title="The first record is one command away" action={<CommandLine text="tare attest" />} />
          <State c="red" word="Not a program" title="This address is not a program" />
          <State c="green" word="None" title="No input has been filed against this record" />
        </div>
      </Section>
    </Page>
  )
}
