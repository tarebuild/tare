import { B } from '@/components/Block'
import { Lockup } from '@/components/Logo'

/** The Open Graph frame, 1200 x 630. Shot from the production server into app/opengraph-image.png. */
export default function OgFrame() {
  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 1000,
        width: 1200,
        height: 630,
        background: '#000',
        display: 'grid',
        gridTemplateRows: 'auto 1fr auto',
        justifyItems: 'center',
        alignItems: 'center',
        padding: 56,
      }}
    >
      <Lockup height={34} />
      <div style={{ display: 'flex', gap: 22, fontFamily: 'var(--mono)', fontSize: 156, fontWeight: 550, letterSpacing: '-0.055em', lineHeight: 1.16 }}>
        <B c="white">Weigh</B>
        <B c="green">less.</B>
      </div>
      <p style={{ fontSize: 30, fontWeight: 500, letterSpacing: '-0.02em', color: '#b4b4bb' }}>
        anchor build becomes tare build. Not a line changed.
      </p>
    </div>
  )
}
