import { Compat, open } from '@tare/contract'
import raw from '@/lib/compatibility.json'
import styles from './docs.module.css'

const parsed = open(Compat).safeParse(raw)
const doc = parsed.success && !parsed.data.fixture ? parsed.data : null

/** The compatibility matrix, generated from backend's test runs (lib/compatibility.json). */
export function Compatibility() {
  if (!doc || doc.constructs.length === 0) {
    return <p>The matrix is written by the benchmark&rsquo;s runs and lands here with them.</p>
  }
  const versions = doc.anchorVersions
  return (
    <div className={`table-scroll ${styles.tableWrap}`}>
      <table className="table">
        <caption className="sr-only">Constructs implemented for each Anchor version</caption>
        <thead>
          <tr>
            <th scope="col">Construct</th>
            {versions.map((v) => (
              <th key={v} scope="col">
                Anchor {v}
              </th>
            ))}
            <th scope="col">Exercised by</th>
          </tr>
        </thead>
        <tbody>
          {doc.constructs.map((c) => {
            const programs = [...new Set(c.versions.flatMap((v) => v.programs))]
            return (
              <tr key={c.construct}>
                <td className="num">{c.construct}</td>
                {versions.map((v) => {
                  const cell = c.versions.find((x) => x.anchorVersion === v)
                  return (
                    <td key={v}>
                      {cell?.supported ? <span className={styles.yes}>Implemented</span> : <span className={styles.no}>In progress</span>}
                    </td>
                  )
                })}
                <td>{programs.length > 0 ? programs.join(', ') : ''}</td>
              </tr>
            )
          })}
        </tbody>
      </table>
    </div>
  )
}
