import { CloseBand, CommandsBand, IdenticalBand, ProofBand, StartBand, WeighingBand } from '@/components/landing/Bands'
import { Hero } from '@/components/landing/Hero'
import { Reveal } from '@/components/Reveal'
import { SmoothScroll } from '@/components/SmoothScroll'
import { registryTotals } from '@/lib/api'

export const revalidate = 60

export default async function Landing() {
  const registry = await registryTotals()
  return (
    <>
      <SmoothScroll />
      <Reveal />
      <Hero />
      <StartBand />
      <WeighingBand />
      <CommandsBand />
      <IdenticalBand />
      <ProofBand />
      <CloseBand registry={registry} />
    </>
  )
}
