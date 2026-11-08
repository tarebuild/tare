import { B } from '@/components/Block'
import { Cascade } from '@/components/Cascade'
import { Glass } from '@/components/Glass'
import { sourceFrom } from '@/lib/source'
import styles from './Hero.module.css'

/** The first view: one column of an Anchor program's source falling down the centre, and one line of it enlarged. */
export function Hero() {
  return (
    <section id="hero" className={styles.hero} aria-labelledby="hero-h">
      <div className={styles.field}>
        <div className={`fall ${styles.stream}`}>
          <Cascade stanzas={sourceFrom(0, 12)} amplitude={0} mode="build" className={styles.col} />
          <Cascade stanzas={sourceFrom(0, 12)} amplitude={0} mode="build" className={styles.col} />
        </div>
      </div>
      <div className={`wrap ${styles.inner}`}>
        <h1 id="hero-h" className={styles.title} data-land>
          <Glass className={styles.glass}>Weigh</Glass>
          <B c="green">less.</B>
        </h1>
      </div>
    </section>
  )
}
