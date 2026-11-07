import { Page, Skeleton } from './ui'

/** The shape of a working page while its data is read. */
export function Loading({ label }: { label: string }) {
  return (
    <Page>
      <div role="status" aria-label={label} style={{ display: 'grid', gap: 32 }}>
        <Skeleton w="min(420px, 70%)" h={44} />
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(160px, 1fr))', gap: 12 }}>
          {[0, 1, 2, 3].map((i) => (
            <Skeleton key={i} h={104} />
          ))}
        </div>
        <div style={{ display: 'grid', gap: 18 }}>
          {[92, 84, 76, 68, 60, 52].map((w) => (
            <Skeleton key={w} w={`${w}%`} h={16} />
          ))}
        </div>
      </div>
    </Page>
  )
}
