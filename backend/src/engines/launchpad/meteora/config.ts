// DBC configs: a curve from the SDK's builder with its migration threshold kept to the unit, the
// instructions that create it, and the curve quoted offline, for sizing a first buy before any
// pool exists.

import BN from 'bn.js'
import type { Connection, PublicKey, TransactionInstruction } from '@solana/web3.js'
import {
  DynamicBondingCurveClient,
  TokenType,
  buildCurve,
  getMigrationThresholdPrice,
  swapQuote,
  type BuildCurveParams,
  type ConfigParameters,
  type QuotePoolConfig,
  type TransferFeeParameters,
} from '@meteora-ag/dynamic-bonding-curve-sdk'
import { withoutComputeBudget } from './instructions.js'
import { big, bn, checkSlippage } from './math.js'
import { quoteMath } from './quote-error.js'

export type ExactCurveParams = Omit<BuildCurveParams, 'migrationQuoteThreshold'> & {
  /** in quote atoms (lamports for SOL) */
  migrationQuoteThreshold: bigint
}

/**
 * buildCurve with the threshold given in atoms and held exact. SDK 2.0.0 reads that field with
 * Decimal and convertToLamports, which take a decimal string; its `number` annotation is narrower
 * than the code, and a float loses the last units. The built config is checked to the unit.
 */
export function buildExactCurve(params: ExactCurveParams): ConfigParameters {
  const { migrationQuoteThreshold: atoms, ...rest } = params
  const decimals = rest.token.tokenQuoteDecimal
  const scale = 10n ** BigInt(decimals)
  const whole = decimals ? `${atoms / scale}.${(atoms % scale).toString().padStart(decimals, '0')}` : atoms.toString()
  const config = buildCurve({ ...rest, migrationQuoteThreshold: whole as unknown as number })
  if (big(config.migrationQuoteThreshold) !== atoms) throw new Error('The SDK curve builder moved the migration threshold.')
  return config
}

/**
 * createConfig2 for a config account that signs at top level (a fresh keypair). A config that is
 * a program PDA is created by that program's CPI instead (program/dbc_cpi.rs).
 */
export async function createConfigInstructions(args: {
  connection: Connection
  parameters: ConfigParameters
  config: PublicKey
  payer: PublicKey
  feeClaimer: PublicKey
  leftoverReceiver: PublicKey
  quoteMint: PublicKey
  transferFeeParameters?: TransferFeeParameters | null
}): Promise<TransactionInstruction[]> {
  const { connection, parameters, transferFeeParameters = null, ...accounts } = args
  const client = new DynamicBondingCurveClient(connection, 'confirmed')
  const tx = await client.partner.createConfig2({ ...parameters, ...accounts, transferFeeParameters })
  return withoutComputeBudget(tx.instructions)
}

/** The quote-relevant part of a config, from its parameters, before the config exists on chain. */
export function quoteConfig(parameters: ConfigParameters, quoteTokenType: TokenType = TokenType.SPLToken): QuotePoolConfig {
  const dynamic = parameters.poolFees.dynamicFee
  return {
    poolFees: {
      baseFee: parameters.poolFees.baseFee,
      dynamicFee: dynamic ? { initialized: 1, binStep: dynamic.binStep, variableFeeControl: dynamic.variableFeeControl } : { initialized: 0, binStep: 0, variableFeeControl: 0 },
    },
    collectFeeMode: parameters.collectFeeMode,
    sqrtStartPrice: parameters.sqrtStartPrice,
    migrationQuoteThreshold: parameters.migrationQuoteThreshold,
    migrationSqrtPrice: getMigrationThresholdPrice(parameters.migrationQuoteThreshold, parameters.sqrtStartPrice, parameters.curve),
    curve: parameters.curve,
    enableFirstSwapWithMinFee: parameters.enableFirstSwapWithMinFee ? 1 : 0,
    quoteTokenFlag: quoteTokenType,
    transferFeeFlag: 0,
    transferFeeBasisPoints: 0,
  }
}

/** Where a curve stands: its sqrt price, and whether it has traded (the first swap's fee rules). */
export type CurveState = { sqrtPrice: bigint; hasSwap: boolean }

export const openingState = (parameters: ConfigParameters): CurveState => ({ sqrtPrice: big(parameters.sqrtStartPrice), hasSwap: false })

/** A swap on the curve's math alone: the first buy of a pool not yet created, or a what-if. */
export function quoteCurve(config: QuotePoolConfig, state: CurveState, side: 'buy' | 'sell', amountIn: bigint, slippageBps = 0) {
  checkSlippage(slippageBps)
  const pool = {
    poolState: {
      sqrtPrice: new BN(state.sqrtPrice.toString()),
      quoteReserve: new BN(0),
      activationPoint: new BN(0),
      hasSwap: state.hasSwap ? 1 : 0,
      volatilityTracker: { volatilityAccumulator: new BN(0) },
    },
  }
  const result = quoteMath(() => swapQuote(pool, config, side === 'sell', bn(amountIn), slippageBps, false, new BN(0), false))
  return {
    amountOut: big(result.outputAmount),
    minimumOut: big(result.minimumAmountOut),
    fee: big(result.tradingFee.add(result.protocolFee).add(result.referralFee)),
    next: { sqrtPrice: big(result.nextSqrtPrice), hasSwap: true } satisfies CurveState,
  }
}
