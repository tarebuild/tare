import type { z } from 'zod'
import { createHttp, type CallOptions, type HttpOptions } from './http.js'
import * as s from './schemas.js'

export function createClient(baseUrl: string, options: Omit<HttpOptions, 'apiError'> = {}) {
  const http = createHttp(baseUrl, { ...options, apiError: s.ApiError })
  return {
    config: (call?: CallOptions) => http.get('/v1/config', s.PublicConfig, call),
    health: (call?: CallOptions) => http.get('/health', s.Health, call),
    bench: (call?: CallOptions) => http.get('/v1/bench', s.BenchResults, call),
    benchTotals: (call?: CallOptions) => http.get('/v1/bench/totals', s.BenchTotals, call),
    compat: (call?: CallOptions) => http.get('/v1/compat', s.Compat, call),
    registryTotals: (call?: CallOptions) => http.get('/v1/registry/totals', s.RegistryTotals, call),
    records: (query: z.input<typeof s.RecordsQuery> = {}, call?: CallOptions) => http.get('/v1/records', s.RecordsPage, { ...call, query }),
    record: (record: string, call?: CallOptions) => http.get(`/v1/records/${encodeURIComponent(record)}`, s.RecordDetail, call),
    recentWeighings: (query: z.input<typeof s.RecentQuery> = {}, call?: CallOptions) => http.get('/v1/weigh', s.RecentWeighings, { ...call, query }),
    weigh: (program: string, call?: CallOptions) => http.get(`/v1/weigh/${encodeURIComponent(program)}`, s.WeighResponse, call),
    uploadReport: (body: z.input<typeof s.UploadProofReport>, call?: CallOptions) => http.post('/v1/reports', s.UploadResult, body, call),
    report: (hash: string, call?: CallOptions) => http.get(`/v1/reports/${encodeURIComponent(hash)}`, s.ProofReport, call),
    uploadFlagInput: (body: z.input<typeof s.UploadFlagInput>, call?: CallOptions) => http.post('/v1/flag-inputs', s.UploadResult, body, call),
    flagInput: (hash: string, call?: CallOptions) => http.get(`/v1/flag-inputs/${encodeURIComponent(hash)}`, s.FlagInput, call),
    token: (call?: CallOptions) => http.get('/v1/token', s.TokenFacts, call),
    recordPageUrl: (siteUrl: string, record: string) => `${siteUrl.replace(/\/+$/, '')}/builds/${encodeURIComponent(record)}`,
  }
}
export type TareClient = ReturnType<typeof createClient>
